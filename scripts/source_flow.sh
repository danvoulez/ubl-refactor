#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
ENV_FILE="${SOURCE_FLOW_ENV_FILE:-${ROOT_DIR}/ops/source_flow.env}"

usage() {
  cat <<'USAGE'
Source flow router (Gitea as Git + S3 local versioned + UBL receipts).

Usage:
  scripts/source_flow.sh <command> [options]

Commands:
  profile                 Print active source-flow profile and key gates
  init-remote             Configure local git remote to Gitea (source of truth)
  mirror-github           Mirror current HEAD to GitHub remote (public-core profile)
  push                    Push current HEAD to Gitea remote
  publish                 Build signed git bundle + upload to versioned S3 + emit UBL chip
  deploy                  Pull signed bundle from S3 and deploy release + optional PM2 reload
  cache-profile           Create shared Node/Cargo cache profile env
  cache-save              Save shared caches to S3 (lock-keyed)
  cache-restore           Restore shared caches from S3 (lock-keyed)

Global options:
  --env <file>            Use alternative env file
  -h, --help              Show help

Examples:
  scripts/source_flow.sh --env ops/source_flow.env profile
  scripts/source_flow.sh --env ops/source_flow.env init-remote
  scripts/source_flow.sh --env ops/source_flow.env mirror-github --branch main
  scripts/source_flow.sh push --branch main
  scripts/source_flow.sh publish
  scripts/source_flow.sh deploy
  scripts/source_flow.sh cache-profile
USAGE
}

log() {
  local lvl="$1"
  shift
  printf '[%s] %s\n' "$lvl" "$*"
}

require_cmd() {
  local c="$1"
  command -v "$c" >/dev/null 2>&1 || {
    log "error" "missing command: $c"
    exit 1
  }
}

run_ublx_submit() {
  local chip_file="$1"
  local response_file="$2"
  local gate_url="$3"

  if command -v "$SOURCE_UBLX_BIN" >/dev/null 2>&1; then
    if "$SOURCE_UBLX_BIN" submit --help >/dev/null 2>&1; then
      local -a cmd
      cmd=(
        "$SOURCE_UBLX_BIN" submit
        --input "$chip_file"
        --gate "$gate_url"
        --output "$response_file"
      )
      if [[ -n "$SOURCE_GATE_API_KEY" ]]; then
        cmd+=(--api-key "$SOURCE_GATE_API_KEY")
      fi
      "${cmd[@]}" >/dev/null
      return 0
    fi
  fi

  if [[ "$SOURCE_UBLX_CARGO_FALLBACK" == "true" ]]; then
    local -a cargo_cmd
    cargo_cmd=(
      cargo run -q -p ubl_cli -- submit
      --input "$chip_file"
      --gate "$gate_url"
      --output "$response_file"
    )
    if [[ -n "$SOURCE_GATE_API_KEY" ]]; then
      cargo_cmd+=(--api-key "$SOURCE_GATE_API_KEY")
    fi
    (cd "$ROOT_DIR" && "${cargo_cmd[@]}" >/dev/null)
    return 0
  fi

  log "error" "ublx not found and SOURCE_UBLX_CARGO_FALLBACK=false"
  return 1
}

sha256_file() {
  local f="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$f" | awk '{print $1}'
  else
    shasum -a 256 "$f" | awk '{print $1}'
  fi
}

now_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

aws_cmd() {
  AWS_EC2_METADATA_DISABLED=true \
  AWS_DEFAULT_REGION="$SOURCE_S3_REGION" \
  aws --no-cli-pager --endpoint-url "$SOURCE_S3_ENDPOINT" --region "$SOURCE_S3_REGION" "$@"
}

ensure_bucket_versioning() {
  if ! aws_cmd s3api head-bucket --bucket "$SOURCE_S3_BUCKET" >/dev/null 2>&1; then
    log "info" "creating bucket: $SOURCE_S3_BUCKET"
    aws_cmd s3api create-bucket --bucket "$SOURCE_S3_BUCKET" >/dev/null
  fi
  aws_cmd s3api put-bucket-versioning \
    --bucket "$SOURCE_S3_BUCKET" \
    --versioning-configuration Status=Enabled >/dev/null
}

ensure_repo_root() {
  git -C "$SOURCE_REPO_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1 || {
    log "error" "not a git repository: $SOURCE_REPO_ROOT"
    exit 1
  }
}

parse_branch_arg() {
  local parsed=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --branch)
        parsed="${2:-}"
        shift 2
        ;;
      *)
        if [[ -z "$parsed" ]]; then
          parsed="$1"
          shift
        else
          log "error" "unknown branch argument: $1"
          exit 1
        fi
        ;;
    esac
  done

  if [[ -z "$parsed" ]]; then
    parsed="$(current_branch)"
  fi
  printf '%s\n' "$parsed"
}

ensure_clean_worktree_if_required() {
  if [[ "$SOURCE_REQUIRE_CLEAN_WORKTREE" == "true" ]]; then
    if [[ -n "$(git -C "$SOURCE_REPO_ROOT" status --porcelain)" ]]; then
      log "error" "worktree is dirty; commit or stash changes before push/publish"
      exit 1
    fi
  fi
}

ensure_signed_head_if_required() {
  local commit="$1"
  if [[ "$SOURCE_REQUIRE_SIGNED_COMMIT" == "true" ]]; then
    if ! git -C "$SOURCE_REPO_ROOT" verify-commit "$commit" >/dev/null 2>&1; then
      log "error" "HEAD commit is not verifiable as signed: $commit"
      exit 1
    fi
  fi
}

resolve_local_binary_trust() {
  TRUST_LOCAL_BIN_SHA256=""
  TRUST_LOCAL_BIN_PATH=""

  if [[ -n "$SOURCE_TRUST_LOCAL_BINARY_PATH" && -f "$SOURCE_TRUST_LOCAL_BINARY_PATH" ]]; then
    TRUST_LOCAL_BIN_PATH="$SOURCE_TRUST_LOCAL_BINARY_PATH"
    TRUST_LOCAL_BIN_SHA256="$(sha256_file "$SOURCE_TRUST_LOCAL_BINARY_PATH")"
    return 0
  fi

  if [[ "$SOURCE_TRUST_LOCAL_BINARY_REQUIRED" == "true" ]]; then
    log "error" "local trust binary missing: $SOURCE_TRUST_LOCAL_BINARY_PATH"
    exit 1
  fi
}

assert_public_core_mirror_state() {
  if [[ "$SOURCE_PROFILE" != "public_core" ]]; then
    return 0
  fi
  if [[ "$SOURCE_PUBLIC_REQUIRE_GITHUB_MIRROR" != "true" ]]; then
    return 0
  fi

  local branch local_commit remote_commit
  branch="$(current_branch)"
  local_commit="$(git -C "$SOURCE_REPO_ROOT" rev-parse HEAD)"
  remote_commit="$(git -C "$SOURCE_REPO_ROOT" ls-remote "$SOURCE_GITHUB_REMOTE_NAME" "refs/heads/${branch}" | awk '{print $1}' | head -n1)"

  if [[ -z "$remote_commit" ]]; then
    log "error" "github mirror branch not found: ${SOURCE_GITHUB_REMOTE_NAME}/${branch}"
    exit 1
  fi
  if [[ "$remote_commit" != "$local_commit" ]]; then
    log "error" "public_core requires GitHub mirror at same HEAD (local=$local_commit remote=$remote_commit)"
    log "info" "run: scripts/source_flow.sh --env <env> mirror-github --branch ${branch}"
    exit 1
  fi
}

current_branch() {
  git -C "$SOURCE_REPO_ROOT" symbolic-ref --short HEAD 2>/dev/null || echo "detached"
}

ensure_signing_material() {
  [[ -f "$SOURCE_SIGNING_KEY_PEM" ]] || {
    log "error" "signing key not found: $SOURCE_SIGNING_KEY_PEM"
    exit 1
  }
  if [[ ! -f "$SOURCE_SIGNING_PUB_PEM" ]]; then
    mkdir -p "$(dirname "$SOURCE_SIGNING_PUB_PEM")"
    openssl pkey -in "$SOURCE_SIGNING_KEY_PEM" -pubout -out "$SOURCE_SIGNING_PUB_PEM" >/dev/null 2>&1
  fi
}

sign_and_verify_manifest() {
  local manifest_file="$1"
  local sig_file="$2"
  ensure_signing_material
  openssl pkeyutl -sign -rawin \
    -inkey "$SOURCE_SIGNING_KEY_PEM" \
    -in "$manifest_file" \
    -out "$sig_file" >/dev/null 2>&1
  openssl pkeyutl -verify -rawin \
    -pubin -inkey "$SOURCE_SIGNING_PUB_PEM" \
    -in "$manifest_file" \
    -sigfile "$sig_file" >/dev/null 2>&1
}

s3_key_exists() {
  local key="$1"
  aws_cmd s3api head-object --bucket "$SOURCE_S3_BUCKET" --key "$key" >/dev/null 2>&1
}

emit_chip() {
  local chip_type="$1"
  local chip_id="$2"
  local payload_file="$3"
  local out_dir="$4"

  if [[ "$SOURCE_EMIT_CHIP" != "true" ]]; then
    log "info" "SOURCE_EMIT_CHIP=false, skipping chip emission"
    return 0
  fi

  local chip_file response_file
  chip_file="$out_dir/${chip_id}.chip.json"
  response_file="$out_dir/${chip_id}.response.json"

  jq -n \
    --arg type "$chip_type" \
    --arg id "$chip_id" \
    --arg ver "1.0" \
    --arg world "$SOURCE_WORLD" \
    --arg emitted_at "$(now_utc)" \
    --slurpfile data "$payload_file" \
    '{
      "@type":$type,
      "@id":$id,
      "@ver":$ver,
      "@world":$world,
      emitted_at:$emitted_at,
      data:$data[0]
    }' > "$chip_file"

  if ! run_ublx_submit "$chip_file" "$response_file" "$SOURCE_GATE_URL"; then
    log "warn" "failed to emit chip to gate: ${SOURCE_GATE_URL}"
    return 0
  fi

  local rcid
  rcid="$(jq -r '.receipt_cid // empty' "$response_file")"
  if [[ -n "$rcid" ]]; then
    log "ok" "chip emitted (${chip_type}) receipt_cid=${rcid}"
  else
    log "warn" "chip emitted but no receipt_cid in response"
  fi
}

cmd_profile() {
  cat <<EOF
profile=${SOURCE_PROFILE}
repo_root=${SOURCE_REPO_ROOT}
gitea_remote=${SOURCE_GITEA_REMOTE_NAME}
github_remote=${SOURCE_GITHUB_REMOTE_NAME}
public_require_mirror=${SOURCE_PUBLIC_REQUIRE_GITHUB_MIRROR}
trust_local_binary_required=${SOURCE_TRUST_LOCAL_BINARY_REQUIRED}
trust_local_binary_path=${SOURCE_TRUST_LOCAL_BINARY_PATH}
EOF
}

cmd_init_remote() {
  ensure_repo_root
  [[ -n "$SOURCE_GITEA_URL" ]] || {
    log "error" "SOURCE_GITEA_URL must be set"
    exit 1
  }

  if git -C "$SOURCE_REPO_ROOT" remote get-url "$SOURCE_GITEA_REMOTE_NAME" >/dev/null 2>&1; then
    git -C "$SOURCE_REPO_ROOT" remote set-url "$SOURCE_GITEA_REMOTE_NAME" "$SOURCE_GITEA_URL"
    log "ok" "updated remote ${SOURCE_GITEA_REMOTE_NAME} -> ${SOURCE_GITEA_URL}"
  else
    git -C "$SOURCE_REPO_ROOT" remote add "$SOURCE_GITEA_REMOTE_NAME" "$SOURCE_GITEA_URL"
    log "ok" "added remote ${SOURCE_GITEA_REMOTE_NAME} -> ${SOURCE_GITEA_URL}"
  fi

  git -C "$SOURCE_REPO_ROOT" config remote.pushDefault "$SOURCE_GITEA_REMOTE_NAME"
  git -C "$SOURCE_REPO_ROOT" remote -v | rg "^${SOURCE_GITEA_REMOTE_NAME}[[:space:]]"
}

cmd_mirror_github() {
  ensure_repo_root
  local branch commit
  branch="$(parse_branch_arg "$@")"
  commit="$(git -C "$SOURCE_REPO_ROOT" rev-parse HEAD)"

  ensure_clean_worktree_if_required
  ensure_signed_head_if_required "$commit"

  if ! git -C "$SOURCE_REPO_ROOT" remote get-url "$SOURCE_GITHUB_REMOTE_NAME" >/dev/null 2>&1; then
    log "error" "github remote not configured: $SOURCE_GITHUB_REMOTE_NAME"
    exit 1
  fi

  git -C "$SOURCE_REPO_ROOT" push "$SOURCE_GITHUB_REMOTE_NAME" "HEAD:refs/heads/${branch}"
  if [[ "$SOURCE_MIRROR_PUSH_TAGS" == "true" ]]; then
    git -C "$SOURCE_REPO_ROOT" push "$SOURCE_GITHUB_REMOTE_NAME" --tags
  fi

  local remote_commit
  remote_commit="$(git -C "$SOURCE_REPO_ROOT" ls-remote "$SOURCE_GITHUB_REMOTE_NAME" "refs/heads/${branch}" | awk '{print $1}' | head -n1)"
  if [[ "$remote_commit" != "$commit" ]]; then
    log "error" "mirror verification failed local=$commit remote=${remote_commit:-<none>}"
    exit 1
  fi

  log "ok" "mirrored HEAD to ${SOURCE_GITHUB_REMOTE_NAME}/${branch}"
}

cmd_push() {
  ensure_repo_root
  local branch
  branch="$(parse_branch_arg "$@")"
  local commit
  commit="$(git -C "$SOURCE_REPO_ROOT" rev-parse HEAD)"

  ensure_clean_worktree_if_required
  ensure_signed_head_if_required "$commit"

  git -C "$SOURCE_REPO_ROOT" push "$SOURCE_GITEA_REMOTE_NAME" "HEAD:refs/heads/${branch}" --follow-tags
  log "ok" "pushed HEAD to ${SOURCE_GITEA_REMOTE_NAME}/${branch}"
}

cmd_publish() {
  ensure_repo_root
  require_cmd jq
  require_cmd aws
  require_cmd openssl

  ensure_clean_worktree_if_required
  assert_public_core_mirror_state
  resolve_local_binary_trust

  ensure_bucket_versioning

  local ts repo_name branch commit tree remote_url github_remote_url stage_dir
  local bundle_file manifest_file manifest_sig latest_file
  local bundle_sha manifest_sha manifest_sig_sha pub_sha
  local bundle_bytes

  ts="$(date -u +%Y%m%dT%H%M%SZ)"
  repo_name="$(basename "$SOURCE_REPO_ROOT")"
  branch="$(current_branch)"
  commit="$(git -C "$SOURCE_REPO_ROOT" rev-parse HEAD)"
  ensure_signed_head_if_required "$commit"
  tree="$(git -C "$SOURCE_REPO_ROOT" rev-parse HEAD^{tree})"
  remote_url="$(git -C "$SOURCE_REPO_ROOT" remote get-url "$SOURCE_GITEA_REMOTE_NAME" 2>/dev/null || true)"
  github_remote_url="$(git -C "$SOURCE_REPO_ROOT" remote get-url "$SOURCE_GITHUB_REMOTE_NAME" 2>/dev/null || true)"

  stage_dir="$ROOT_DIR/artifacts/source-flow/publish-${ts}-${commit:0:12}"
  mkdir -p "$stage_dir"
  bundle_file="$stage_dir/source.bundle"
  manifest_file="$stage_dir/manifest.json"
  manifest_sig="$stage_dir/manifest.sig"
  latest_file="$stage_dir/latest.json"

  git -C "$SOURCE_REPO_ROOT" bundle create "$bundle_file" "HEAD"
  git bundle verify "$bundle_file" >/dev/null 2>&1 || true

  bundle_sha="$(sha256_file "$bundle_file")"
  bundle_bytes="$(wc -c < "$bundle_file" | tr -d ' ')"

  jq -n \
    --arg created_at "$(now_utc)" \
    --arg repo_name "$repo_name" \
    --arg branch "$branch" \
    --arg commit "$commit" \
    --arg tree "$tree" \
    --arg remote_name "$SOURCE_GITEA_REMOTE_NAME" \
    --arg remote_url "$remote_url" \
    --arg github_remote_name "$SOURCE_GITHUB_REMOTE_NAME" \
    --arg github_remote_url "$github_remote_url" \
    --arg bundle_sha256 "$bundle_sha" \
    --argjson bundle_bytes "$bundle_bytes" \
    --arg s3_bucket "$SOURCE_S3_BUCKET" \
    --arg s3_prefix "$SOURCE_S3_PREFIX" \
    --arg namespace "$SOURCE_REPO_NAMESPACE" \
    --arg profile "$SOURCE_PROFILE" \
    --arg trust_local_binary_path "$TRUST_LOCAL_BIN_PATH" \
    --arg trust_local_binary_sha256 "$TRUST_LOCAL_BIN_SHA256" \
    '{
      created_at:$created_at,
      repo:{
        name:$repo_name,namespace:$namespace,remote_name:$remote_name,remote_url:$remote_url,
        github_remote_name:$github_remote_name,github_remote_url:$github_remote_url,
        branch:$branch,commit:$commit,tree:$tree
      },
      artifact:{kind:"git.bundle",bundle_sha256:$bundle_sha256,bundle_bytes:$bundle_bytes},
      storage:{bucket:$s3_bucket,prefix:$s3_prefix},
      profile:$profile,
      trust_local_binary:{path:$trust_local_binary_path,sha256:$trust_local_binary_sha256}
    }' | jq -S '.' > "$manifest_file"

  sign_and_verify_manifest "$manifest_file" "$manifest_sig"

  manifest_sha="$(sha256_file "$manifest_file")"
  manifest_sig_sha="$(sha256_file "$manifest_sig")"
  pub_sha="$(sha256_file "$SOURCE_SIGNING_PUB_PEM")"

  local repo_key latest_key
  repo_key="${SOURCE_S3_PREFIX}/${SOURCE_REPO_NAMESPACE}/${repo_name}/${commit}"
  latest_key="${SOURCE_S3_PREFIX}/${SOURCE_REPO_NAMESPACE}/${repo_name}/latest.json"

  aws_cmd s3 cp "$bundle_file" "s3://${SOURCE_S3_BUCKET}/${repo_key}/source.bundle" >/dev/null
  aws_cmd s3 cp "$manifest_file" "s3://${SOURCE_S3_BUCKET}/${repo_key}/manifest.json" >/dev/null
  aws_cmd s3 cp "$manifest_sig" "s3://${SOURCE_S3_BUCKET}/${repo_key}/manifest.sig" >/dev/null
  aws_cmd s3 cp "$SOURCE_SIGNING_PUB_PEM" "s3://${SOURCE_S3_BUCKET}/${repo_key}/signer.pub.pem" >/dev/null

  jq -n \
    --arg updated_at "$(now_utc)" \
    --arg repo_name "$repo_name" \
    --arg namespace "$SOURCE_REPO_NAMESPACE" \
    --arg commit "$commit" \
    --arg branch "$branch" \
    --arg bundle_sha256 "$bundle_sha" \
    --arg manifest_sha256 "$manifest_sha" \
    --arg signer_pub_sha256 "$pub_sha" \
    --arg bundle_key "${repo_key}/source.bundle" \
    --arg manifest_key "${repo_key}/manifest.json" \
    --arg sig_key "${repo_key}/manifest.sig" \
    --arg pub_key "${repo_key}/signer.pub.pem" \
    '{
      updated_at:$updated_at,
      repo:{name:$repo_name,namespace:$namespace,branch:$branch,commit:$commit},
      artifact:{
        bundle_key:$bundle_key,
        manifest_key:$manifest_key,
        sig_key:$sig_key,
        pub_key:$pub_key,
        bundle_sha256:$bundle_sha256,
        manifest_sha256:$manifest_sha256,
        signer_pub_sha256:$signer_pub_sha256
      }
    }' | jq -S '.' > "$latest_file"

  aws_cmd s3 cp "$latest_file" "s3://${SOURCE_S3_BUCKET}/${latest_key}" >/dev/null

  local publish_report
  publish_report="$stage_dir/publish.report.json"
  jq -n \
    --arg created_at "$(now_utc)" \
    --arg repo_name "$repo_name" \
    --arg branch "$branch" \
    --arg commit "$commit" \
    --arg bucket "$SOURCE_S3_BUCKET" \
    --arg latest_key "$latest_key" \
    --arg bundle_key "${repo_key}/source.bundle" \
    --arg manifest_key "${repo_key}/manifest.json" \
    --arg sig_key "${repo_key}/manifest.sig" \
    --arg pub_key "${repo_key}/signer.pub.pem" \
    --arg bundle_sha256 "$bundle_sha" \
    --arg manifest_sha256 "$manifest_sha" \
    --arg manifest_sig_sha256 "$manifest_sig_sha" \
    --arg signer_pub_sha256 "$pub_sha" \
    --arg profile "$SOURCE_PROFILE" \
    --arg trust_local_binary_path "$TRUST_LOCAL_BIN_PATH" \
    --arg trust_local_binary_sha256 "$TRUST_LOCAL_BIN_SHA256" \
    '{
      created_at:$created_at,
      repo:{name:$repo_name,branch:$branch,commit:$commit},
      profile:$profile,
      storage:{
        bucket:$bucket,
        latest_key:$latest_key,
        bundle_key:$bundle_key,
        manifest_key:$manifest_key,
        sig_key:$sig_key,
        pub_key:$pub_key
      },
      hashes:{
        bundle_sha256:$bundle_sha256,
        manifest_sha256:$manifest_sha256,
        manifest_sig_sha256:$manifest_sig_sha256,
        signer_pub_sha256:$signer_pub_sha256
      },
      trust_local_binary:{path:$trust_local_binary_path,sha256:$trust_local_binary_sha256}
    }' > "$publish_report"

  emit_chip "ops/source.publish.v1" "source-publish-${repo_name}-${ts}" "$publish_report" "$stage_dir"

  log "ok" "published signed source bundle"
  log "ok" "latest pointer: s3://${SOURCE_S3_BUCKET}/${latest_key}"
  log "ok" "report: ${publish_report}"
}

cmd_deploy() {
  ensure_repo_root
  require_cmd jq
  require_cmd aws
  require_cmd openssl
  require_cmd git
  resolve_local_binary_trust

  local commit="${1:-}"
  local repo_name latest_key stage_dir latest_file
  local repo_key bundle_key manifest_key sig_key pub_key
  local bundle_file manifest_file sig_file pub_file
  local deploy_commit release_dir

  repo_name="$(basename "$SOURCE_REPO_ROOT")"
  latest_key="${SOURCE_S3_PREFIX}/${SOURCE_REPO_NAMESPACE}/${repo_name}/latest.json"
  stage_dir="$ROOT_DIR/artifacts/source-flow/deploy-$(date -u +%Y%m%dT%H%M%SZ)"
  mkdir -p "$stage_dir"
  latest_file="$stage_dir/latest.json"

  aws_cmd s3 cp "s3://${SOURCE_S3_BUCKET}/${latest_key}" "$latest_file" >/dev/null

  if [[ -n "$commit" ]]; then
    deploy_commit="$commit"
    repo_key="${SOURCE_S3_PREFIX}/${SOURCE_REPO_NAMESPACE}/${repo_name}/${deploy_commit}"
    bundle_key="${repo_key}/source.bundle"
    manifest_key="${repo_key}/manifest.json"
    sig_key="${repo_key}/manifest.sig"
    pub_key="${repo_key}/signer.pub.pem"
  else
    deploy_commit="$(jq -r '.repo.commit' "$latest_file")"
    bundle_key="$(jq -r '.artifact.bundle_key' "$latest_file")"
    manifest_key="$(jq -r '.artifact.manifest_key' "$latest_file")"
    sig_key="$(jq -r '.artifact.sig_key' "$latest_file")"
    pub_key="$(jq -r '.artifact.pub_key' "$latest_file")"
  fi

  bundle_file="$stage_dir/source.bundle"
  manifest_file="$stage_dir/manifest.json"
  sig_file="$stage_dir/manifest.sig"
  pub_file="$stage_dir/signer.pub.pem"

  aws_cmd s3 cp "s3://${SOURCE_S3_BUCKET}/${bundle_key}" "$bundle_file" >/dev/null
  aws_cmd s3 cp "s3://${SOURCE_S3_BUCKET}/${manifest_key}" "$manifest_file" >/dev/null
  aws_cmd s3 cp "s3://${SOURCE_S3_BUCKET}/${sig_key}" "$sig_file" >/dev/null
  aws_cmd s3 cp "s3://${SOURCE_S3_BUCKET}/${pub_key}" "$pub_file" >/dev/null

  local expected_bundle_sha actual_bundle_sha
  expected_bundle_sha="$(jq -r '.artifact.bundle_sha256' "$manifest_file")"
  actual_bundle_sha="$(sha256_file "$bundle_file")"
  if [[ "$expected_bundle_sha" != "$actual_bundle_sha" ]]; then
    log "error" "bundle sha mismatch expected=$expected_bundle_sha actual=$actual_bundle_sha"
    exit 1
  fi

  openssl pkeyutl -verify -rawin -pubin \
    -inkey "$pub_file" \
    -in "$manifest_file" \
    -sigfile "$sig_file" >/dev/null 2>&1

  release_dir="${SOURCE_DEPLOY_ROOT}/releases/${deploy_commit}"
  mkdir -p "${SOURCE_DEPLOY_ROOT}/releases"
  if [[ ! -d "$release_dir/.git" ]]; then
    git clone "$bundle_file" "$release_dir" >/dev/null 2>&1
  fi
  git -C "$release_dir" checkout "$deploy_commit" >/dev/null 2>&1
  ln -sfn "$release_dir" "${SOURCE_DEPLOY_ROOT}/current"

  if [[ -n "$SOURCE_DEPLOY_BUILD_CMD" ]]; then
    (cd "$release_dir" && bash -lc "$SOURCE_DEPLOY_BUILD_CMD")
  fi
  if [[ -n "$SOURCE_PM2_RELOAD_CMD" ]]; then
    bash -lc "$SOURCE_PM2_RELOAD_CMD"
  fi

  local deploy_report
  deploy_report="$stage_dir/deploy.report.json"
  jq -n \
    --arg deployed_at "$(now_utc)" \
    --arg repo_name "$repo_name" \
    --arg commit "$deploy_commit" \
    --arg release_dir "$release_dir" \
    --arg current_link "${SOURCE_DEPLOY_ROOT}/current" \
    --arg bundle_sha256 "$actual_bundle_sha" \
    --arg manifest_sha256 "$(sha256_file "$manifest_file")" \
    --arg signature_sha256 "$(sha256_file "$sig_file")" \
    --arg profile "$SOURCE_PROFILE" \
    --arg trust_local_binary_path "$TRUST_LOCAL_BIN_PATH" \
    --arg trust_local_binary_sha256 "$TRUST_LOCAL_BIN_SHA256" \
    '{
      deployed_at:$deployed_at,
      repo:{name:$repo_name,commit:$commit},
      profile:$profile,
      deploy:{release_dir:$release_dir,current_link:$current_link},
      verified_hashes:{
        bundle_sha256:$bundle_sha256,
        manifest_sha256:$manifest_sha256,
        signature_sha256:$signature_sha256
      },
      trust_local_binary:{path:$trust_local_binary_path,sha256:$trust_local_binary_sha256}
    }' > "$deploy_report"

  emit_chip "ops/source.deploy.v1" "source-deploy-${repo_name}-$(date -u +%Y%m%dT%H%M%SZ)" "$deploy_report" "$stage_dir"

  log "ok" "deployed ${repo_name}@${deploy_commit} to ${release_dir}"
}

cmd_cache_profile() {
  ensure_repo_root
  require_cmd jq
  mkdir -p "$SOURCE_CACHE_ROOT"/{npm-cache,pnpm-store,cargo-home,cargo-target}

  local repo_name
  repo_name="$(basename "$SOURCE_REPO_ROOT")"
  mkdir -p "$(dirname "$SOURCE_CACHE_ENV_OUT")"
  cat > "$SOURCE_CACHE_ENV_OUT" <<EOF
export NPM_CONFIG_CACHE="${SOURCE_CACHE_ROOT}/npm-cache"
export PNPM_STORE_PATH="${SOURCE_CACHE_ROOT}/pnpm-store"
export CARGO_HOME="${SOURCE_CACHE_ROOT}/cargo-home"
export CARGO_TARGET_DIR="${SOURCE_CACHE_ROOT}/cargo-target/${repo_name}"
EOF
  chmod 600 "$SOURCE_CACHE_ENV_OUT"

  local report_file out_dir ts
  ts="$(date -u +%Y%m%dT%H%M%SZ)"
  out_dir="$ROOT_DIR/artifacts/source-flow/cache-profile-${ts}"
  mkdir -p "$out_dir"
  report_file="$out_dir/cache.profile.json"
  jq -n \
    --arg created_at "$(now_utc)" \
    --arg env_file "$SOURCE_CACHE_ENV_OUT" \
    --arg cache_root "$SOURCE_CACHE_ROOT" \
    --arg repo_root "$SOURCE_REPO_ROOT" \
    '{
      created_at:$created_at,
      cache_root:$cache_root,
      repo_root:$repo_root,
      env_file:$env_file
    }' > "$report_file"
  emit_chip "ops/cache.profile.v1" "cache-profile-${ts}" "$report_file" "$out_dir"
  log "ok" "cache profile written: $SOURCE_CACHE_ENV_OUT"
}

find_js_lock() {
  local base="$1"
  for f in pnpm-lock.yaml package-lock.json yarn.lock; do
    if [[ -f "$base/$f" ]]; then
      echo "$f"
      return 0
    fi
  done
  echo ""
}

cmd_cache_save() {
  ensure_repo_root
  require_cmd aws
  require_cmd jq
  ensure_bucket_versioning

  local js_lock cargo_lock repo_name ts out_dir report_file
  local js_hash cargo_hash
  local js_tar cargo_tar
  ts="$(date -u +%Y%m%dT%H%M%SZ)"
  out_dir="$ROOT_DIR/artifacts/source-flow/cache-save-${ts}"
  mkdir -p "$out_dir"
  report_file="$out_dir/cache.save.json"
  repo_name="$(basename "$SOURCE_REPO_ROOT")"
  js_lock="$(find_js_lock "$SOURCE_REPO_ROOT")"
  cargo_lock=""
  [[ -f "$SOURCE_REPO_ROOT/Cargo.lock" ]] && cargo_lock="Cargo.lock"

  js_hash=""
  cargo_hash=""
  js_tar=""
  cargo_tar=""

  if [[ -n "$js_lock" && "$SOURCE_CACHE_INCLUDE_NODE_MODULES" == "true" && -d "$SOURCE_REPO_ROOT/node_modules" ]]; then
    js_hash="$(sha256_file "$SOURCE_REPO_ROOT/$js_lock")"
    js_tar="$out_dir/node_modules.${js_hash}.tar.gz"
    tar -C "$SOURCE_REPO_ROOT" -czf "$js_tar" node_modules
    aws_cmd s3 cp "$js_tar" \
      "s3://${SOURCE_S3_BUCKET}/cache/${SOURCE_REPO_NAMESPACE}/${repo_name}/node_modules/${js_hash}.tar.gz" >/dev/null
  fi

  if [[ -n "$cargo_lock" ]]; then
    local cargo_home="${CARGO_HOME:-$HOME/.cargo}"
    if [[ -d "$cargo_home/registry" || -d "$cargo_home/git" ]]; then
      cargo_hash="$(sha256_file "$SOURCE_REPO_ROOT/$cargo_lock")"
      cargo_tar="$out_dir/cargo-home.${cargo_hash}.tar.gz"
      mkdir -p "$out_dir/cargo-stage"
      if [[ -d "$cargo_home/registry" ]]; then
        cp -a "$cargo_home/registry" "$out_dir/cargo-stage/registry"
      fi
      if [[ -d "$cargo_home/git" ]]; then
        cp -a "$cargo_home/git" "$out_dir/cargo-stage/git"
      fi
      tar -C "$out_dir/cargo-stage" -czf "$cargo_tar" .
      aws_cmd s3 cp "$cargo_tar" \
        "s3://${SOURCE_S3_BUCKET}/cache/${SOURCE_REPO_NAMESPACE}/${repo_name}/cargo-home/${cargo_hash}.tar.gz" >/dev/null
    fi
  fi

  jq -n \
    --arg saved_at "$(now_utc)" \
    --arg repo "$repo_name" \
    --arg js_lock "${js_lock:-}" \
    --arg js_hash "${js_hash:-}" \
    --arg cargo_lock "${cargo_lock:-}" \
    --arg cargo_hash "${cargo_hash:-}" \
    --arg include_node_modules "$SOURCE_CACHE_INCLUDE_NODE_MODULES" \
    '{
      saved_at:$saved_at,
      repo:$repo,
      include_node_modules:($include_node_modules=="true"),
      js:{lock_file:$js_lock,lock_sha256:$js_hash},
      rust:{lock_file:$cargo_lock,lock_sha256:$cargo_hash}
    }' > "$report_file"

  emit_chip "ops/cache.save.v1" "cache-save-${repo_name}-${ts}" "$report_file" "$out_dir"
  log "ok" "cache save completed"
}

cmd_cache_restore() {
  ensure_repo_root
  require_cmd aws
  require_cmd jq

  local repo_name js_lock cargo_lock js_hash cargo_hash
  local js_key cargo_key js_tmp cargo_tmp
  local ts out_dir report_file

  ts="$(date -u +%Y%m%dT%H%M%SZ)"
  out_dir="$ROOT_DIR/artifacts/source-flow/cache-restore-${ts}"
  mkdir -p "$out_dir"
  report_file="$out_dir/cache.restore.json"

  repo_name="$(basename "$SOURCE_REPO_ROOT")"
  js_lock="$(find_js_lock "$SOURCE_REPO_ROOT")"
  cargo_lock=""
  [[ -f "$SOURCE_REPO_ROOT/Cargo.lock" ]] && cargo_lock="Cargo.lock"

  js_hash=""
  cargo_hash=""

  if [[ -n "$js_lock" && "$SOURCE_CACHE_INCLUDE_NODE_MODULES" == "true" ]]; then
    js_hash="$(sha256_file "$SOURCE_REPO_ROOT/$js_lock")"
    js_key="cache/${SOURCE_REPO_NAMESPACE}/${repo_name}/node_modules/${js_hash}.tar.gz"
    if s3_key_exists "$js_key"; then
      js_tmp="$out_dir/node_modules.tar.gz"
      aws_cmd s3 cp "s3://${SOURCE_S3_BUCKET}/${js_key}" "$js_tmp" >/dev/null
      rm -rf "$SOURCE_REPO_ROOT/node_modules"
      tar -C "$SOURCE_REPO_ROOT" -xzf "$js_tmp"
      log "ok" "restored node_modules cache"
    else
      log "warn" "node_modules cache not found for hash ${js_hash}"
    fi
  fi

  if [[ -n "$cargo_lock" ]]; then
    local cargo_home="${CARGO_HOME:-$HOME/.cargo}"
    cargo_hash="$(sha256_file "$SOURCE_REPO_ROOT/$cargo_lock")"
    cargo_key="cache/${SOURCE_REPO_NAMESPACE}/${repo_name}/cargo-home/${cargo_hash}.tar.gz"
    if s3_key_exists "$cargo_key"; then
      cargo_tmp="$out_dir/cargo-home.tar.gz"
      aws_cmd s3 cp "s3://${SOURCE_S3_BUCKET}/${cargo_key}" "$cargo_tmp" >/dev/null
      mkdir -p "$cargo_home"
      tar -C "$cargo_home" -xzf "$cargo_tmp"
      log "ok" "restored cargo-home cache"
    else
      log "warn" "cargo-home cache not found for hash ${cargo_hash}"
    fi
  fi

  jq -n \
    --arg restored_at "$(now_utc)" \
    --arg repo "$repo_name" \
    --arg js_lock "${js_lock:-}" \
    --arg js_hash "${js_hash:-}" \
    --arg cargo_lock "${cargo_lock:-}" \
    --arg cargo_hash "${cargo_hash:-}" \
    '{
      restored_at:$restored_at,
      repo:$repo,
      js:{lock_file:$js_lock,lock_sha256:$js_hash},
      rust:{lock_file:$cargo_lock,lock_sha256:$cargo_hash}
    }' > "$report_file"

  emit_chip "ops/cache.restore.v1" "cache-restore-${repo_name}-${ts}" "$report_file" "$out_dir"
  log "ok" "cache restore completed"
}

# parse global options first
while [[ $# -gt 0 ]]; do
  case "$1" in
    --env)
      ENV_FILE="${2:-}"
      shift 2
      ;;
    -h|--help|help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    *)
      break
      ;;
  esac
done

CMD="${1:-}"
if [[ -z "$CMD" || "$CMD" == "-h" || "$CMD" == "--help" || "$CMD" == "help" ]]; then
  usage
  exit 0
fi
shift || true

if [[ -f "$ENV_FILE" ]]; then
  # shellcheck disable=SC1090
  set -a && source "$ENV_FILE" && set +a
fi

SOURCE_REPO_ROOT="${SOURCE_REPO_ROOT:-$ROOT_DIR}"
SOURCE_PROFILE="${SOURCE_PROFILE:-private_product}"
SOURCE_GITEA_REMOTE_NAME="${SOURCE_GITEA_REMOTE_NAME:-gitea}"
SOURCE_GITEA_URL="${SOURCE_GITEA_URL:-}"
SOURCE_GITHUB_REMOTE_NAME="${SOURCE_GITHUB_REMOTE_NAME:-origin}"
SOURCE_PUBLIC_REQUIRE_GITHUB_MIRROR="${SOURCE_PUBLIC_REQUIRE_GITHUB_MIRROR:-true}"
SOURCE_MIRROR_PUSH_TAGS="${SOURCE_MIRROR_PUSH_TAGS:-true}"
SOURCE_REQUIRE_CLEAN_WORKTREE="${SOURCE_REQUIRE_CLEAN_WORKTREE:-true}"
SOURCE_REQUIRE_SIGNED_COMMIT="${SOURCE_REQUIRE_SIGNED_COMMIT:-false}"
SOURCE_SIGNING_KEY_PEM="${SOURCE_SIGNING_KEY_PEM:-$HOME/.logline-keys/source_signing_ed25519.pem}"
SOURCE_SIGNING_PUB_PEM="${SOURCE_SIGNING_PUB_PEM:-$HOME/.logline-keys/source_signing_ed25519.pub.pem}"
SOURCE_S3_ENDPOINT="${SOURCE_S3_ENDPOINT:-http://127.0.0.1:9000}"
SOURCE_S3_REGION="${SOURCE_S3_REGION:-us-east-1}"
SOURCE_S3_BUCKET="${SOURCE_S3_BUCKET:-ubl-source}"
SOURCE_S3_PREFIX="${SOURCE_S3_PREFIX:-repos}"
SOURCE_REPO_NAMESPACE="${SOURCE_REPO_NAMESPACE:-logline}"
SOURCE_GATE_URL="${SOURCE_GATE_URL:-http://127.0.0.1:4000}"
SOURCE_WORLD="${SOURCE_WORLD:-a/chip-registry/t/logline}"
SOURCE_GATE_API_KEY="${SOURCE_GATE_API_KEY:-}"
SOURCE_EMIT_CHIP="${SOURCE_EMIT_CHIP:-true}"
SOURCE_DEPLOY_ROOT="${SOURCE_DEPLOY_ROOT:-$HOME/srv/source-deploy}"
SOURCE_DEPLOY_BUILD_CMD="${SOURCE_DEPLOY_BUILD_CMD:-}"
SOURCE_PM2_RELOAD_CMD="${SOURCE_PM2_RELOAD_CMD:-}"
SOURCE_CACHE_ROOT="${SOURCE_CACHE_ROOT:-$HOME/srv/shared-cache}"
SOURCE_CACHE_ENV_OUT="${SOURCE_CACHE_ENV_OUT:-$ROOT_DIR/ops/source_cache.env}"
SOURCE_CACHE_INCLUDE_NODE_MODULES="${SOURCE_CACHE_INCLUDE_NODE_MODULES:-false}"
SOURCE_UBLX_BIN="${SOURCE_UBLX_BIN:-ublx}"
SOURCE_UBLX_CARGO_FALLBACK="${SOURCE_UBLX_CARGO_FALLBACK:-true}"
SOURCE_TRUST_LOCAL_BINARY_PATH="${SOURCE_TRUST_LOCAL_BINARY_PATH:-/Users/ubl-ops/ubl-core-forever/live/current/bin/ubl_gate}"
SOURCE_TRUST_LOCAL_BINARY_REQUIRED="${SOURCE_TRUST_LOCAL_BINARY_REQUIRED:-false}"

case "$SOURCE_PROFILE" in
  public_core|private_product)
    ;;
  *)
    log "error" "invalid SOURCE_PROFILE=${SOURCE_PROFILE} (expected public_core|private_product)"
    exit 1
    ;;
esac

require_cmd bash
require_cmd git
require_cmd jq

case "$CMD" in
  profile)
    cmd_profile "$@"
    ;;
  init-remote)
    cmd_init_remote "$@"
    ;;
  mirror-github)
    cmd_mirror_github "$@"
    ;;
  push)
    cmd_push "$@"
    ;;
  publish)
    cmd_publish "$@"
    ;;
  deploy)
    cmd_deploy "$@"
    ;;
  cache-profile)
    cmd_cache_profile "$@"
    ;;
  cache-save)
    cmd_cache_save "$@"
    ;;
  cache-restore)
    cmd_cache_restore "$@"
    ;;
  *)
    log "error" "unknown command: $CMD"
    usage
    exit 1
    ;;
esac
