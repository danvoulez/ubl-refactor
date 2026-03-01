#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

sha256_file() {
  local f="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$f" | awk '{print $1}'
  else
    shasum -a 256 "$f" | awk '{print $1}'
  fi
}

sha256_stdin() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  else
    shasum -a 256 | awk '{print $1}'
  fi
}

now_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

usage() {
  cat <<'USAGE'
Usage:
  scripts/docs_attest.sh <command> [options]

Commands:
  init-key        Generate Ed25519 keypair for docs attestation
  build-manifest  Build deterministic docs manifest
  sign            Sign docs manifest and produce attestation bundle
  verify          Verify docs attestation bundle

Examples:
  scripts/docs_attest.sh init-key --key-out ~/.ubl-core/keys/docs_attest_ed25519.pem
  scripts/docs_attest.sh build-manifest --out ./release-artifacts/docs/manifest.json
  scripts/docs_attest.sh sign --manifest ./release-artifacts/docs/manifest.json --key ~/.ubl-core/keys/docs_attest_ed25519.pem --pub ./security/attestation/public_keys/main.pub.pem --out ./release-artifacts/docs/attestation.json
  scripts/docs_attest.sh verify --manifest ./release-artifacts/docs/manifest.json --attestation ./release-artifacts/docs/attestation.json
USAGE
}

collect_attested_files() {
  {
    for p in \
      README.md \
      LICENSE \
      NOTICE \
      COPYRIGHT \
      GOVERNANCE.md \
      CONTRIBUTING.md \
      SECURITY.md \
      SUPPORT.md \
      RFC_PROCESS.md \
      VERSIONING.md \
      COMPATIBILITY.md \
      TRADEMARK_POLICY.md \
      COMMERCIAL-LICENSING.md \
      CODE_OF_CONDUCT.md; do
      [[ -f "$p" ]] && printf '%s\n' "$p"
    done

    if [[ -d docs ]]; then
      find docs -type f \( -name '*.md' -o -name '*.json' -o -name '*.yaml' -o -name '*.yml' \)
    fi

    for p in schemas/unc-1.schema.json kats/unc1/unc1_kats.v1.json; do
      [[ -f "$p" ]] && printf '%s\n' "$p"
    done
  } | LC_ALL=C sort -u
}

cmd_init_key() {
  local key_out="${HOME}/.ubl-core/keys/docs_attest_ed25519.pem"
  local pub_out=""
  local force="false"

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --key-out)
        key_out="${2:-}"
        shift 2
        ;;
      --pub-out)
        pub_out="${2:-}"
        shift 2
        ;;
      --force)
        force="true"
        shift
        ;;
      -h|--help)
        cat <<'USAGE'
Usage: scripts/docs_attest.sh init-key [--key-out <path>] [--pub-out <path>] [--force]

If DOCS_ATTEST_KEY_PASSPHRASE is set, key is encrypted (recommended).
USAGE
        return 0
        ;;
      *)
        echo "[error] unknown arg for init-key: $1" >&2
        return 1
        ;;
    esac
  done

  if [[ -z "$pub_out" ]]; then
    pub_out="${key_out%.pem}.pub.pem"
  fi

  mkdir -p "$(dirname "$key_out")" "$(dirname "$pub_out")"
  if [[ -f "$key_out" && "$force" != "true" ]]; then
    echo "[error] key exists: $key_out (use --force to overwrite)" >&2
    return 1
  fi

  if [[ -n "${DOCS_ATTEST_KEY_PASSPHRASE:-}" ]]; then
    openssl genpkey \
      -algorithm Ed25519 \
      -aes-256-cbc \
      -pass "pass:${DOCS_ATTEST_KEY_PASSPHRASE}" \
      -out "$key_out"
    openssl pkey \
      -in "$key_out" \
      -passin "pass:${DOCS_ATTEST_KEY_PASSPHRASE}" \
      -pubout \
      -out "$pub_out"
    echo "[ok] encrypted keypair generated"
  else
    openssl genpkey -algorithm Ed25519 -out "$key_out"
    openssl pkey -in "$key_out" -pubout -out "$pub_out"
    echo "[warn] key generated without passphrase. Set DOCS_ATTEST_KEY_PASSPHRASE for encrypted key."
  fi

  chmod 600 "$key_out"
  chmod 644 "$pub_out"

  local key_id
  key_id="sha256:$(openssl pkey -pubin -in "$pub_out" -outform DER | sha256_stdin)"
  echo "[ok] private key: $key_out"
  echo "[ok] public key:  $pub_out"
  echo "[ok] key id:      $key_id"
}

cmd_build_manifest() {
  local out="./release-artifacts/docs/manifest.json"
  local repo=""
  local commit=""
  local tag=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --out)
        out="${2:-}"
        shift 2
        ;;
      --repo)
        repo="${2:-}"
        shift 2
        ;;
      --commit)
        commit="${2:-}"
        shift 2
        ;;
      --tag)
        tag="${2:-}"
        shift 2
        ;;
      -h|--help)
        cat <<'USAGE'
Usage: scripts/docs_attest.sh build-manifest [--out <path>] [--repo <name>] [--commit <sha>] [--tag <tag>]
USAGE
        return 0
        ;;
      *)
        echo "[error] unknown arg for build-manifest: $1" >&2
        return 1
        ;;
    esac
  done

  cd "$ROOT_DIR"
  mkdir -p "$(dirname "$out")"

  if [[ -z "$repo" ]]; then
    repo="$(git config --get remote.origin.url || true)"
    [[ -z "$repo" ]] && repo="unknown"
  fi
  if [[ -z "$commit" ]]; then
    commit="$(git rev-parse HEAD)"
  fi
  if [[ -z "$tag" ]]; then
    tag="$(git describe --tags --exact-match 2>/dev/null || true)"
  fi

  local tmp
  tmp="$(mktemp -d)"

  local entries_tsv="$tmp/entries.tsv"
  local entries_jsonl="$tmp/entries.jsonl"
  local tree_txt="$tmp/tree.txt"
  : > "$entries_tsv"
  : > "$entries_jsonl"
  : > "$tree_txt"

  while IFS= read -r path; do
    [[ -f "$path" ]] || continue
    local sha size
    sha="$(sha256_file "$path")"
    size="$(wc -c < "$path" | tr -d '[:space:]')"
    printf '%s\t%s\t%s\n' "$path" "$sha" "$size" >> "$entries_tsv"
    printf '%s  %s\n' "$sha" "$path" >> "$tree_txt"
    jq -nc \
      --arg path "$path" \
      --arg sha "$sha" \
      --argjson size "$size" \
      '{"path":$path,"sha256":$sha,"size_bytes":$size}' >> "$entries_jsonl"
  done < <(collect_attested_files)

  local file_count tree_sha generated_at
  file_count="$(wc -l < "$entries_tsv" | tr -d '[:space:]')"
  tree_sha="$(sha256_file "$tree_txt")"
  generated_at="$(now_utc)"

  if [[ -s "$entries_jsonl" ]]; then
    jq -s '.' "$entries_jsonl" > "$tmp/files.json"
  else
    echo '[]' > "$tmp/files.json"
  fi

  jq -n \
    --arg atype "ubl/docs-manifest" \
    --arg aver "1.0" \
    --arg aid "docs-manifest:${commit}" \
    --arg world "a/logline/t/oss" \
    --arg repo "$repo" \
    --arg generated_at "$generated_at" \
    --arg commit "$commit" \
    --arg tag "$tag" \
    --argjson file_count "$file_count" \
    --arg tree_sha "$tree_sha" \
    --slurpfile files "$tmp/files.json" \
    '{
      "@type": $atype,
      "@id": $aid,
      "@ver": $aver,
      "@world": $world,
      "repo": $repo,
      "generated_at": $generated_at,
      "git": {
        "commit": $commit,
        "tag": (if $tag == "" then null else $tag end)
      },
      "summary": {
        "file_count": $file_count,
        "tree_sha256": $tree_sha
      },
      "files": $files[0]
    }' > "$out"

  rm -rf "$tmp"

  echo "[ok] docs manifest written: $out"
  echo "[ok] files: $file_count"
  echo "[ok] tree_sha256: $tree_sha"
}

cmd_sign() {
  local manifest=""
  local key=""
  local pub=""
  local out="./release-artifacts/docs/attestation.json"
  local sig_out=""
  local tag=""
  local commit=""
  local require_ubl_cid="false"
  local with_ubl_cid="${DOCS_ATTEST_WITH_UBL_CID:-false}"

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --manifest)
        manifest="${2:-}"
        shift 2
        ;;
      --key)
        key="${2:-}"
        shift 2
        ;;
      --pub)
        pub="${2:-}"
        shift 2
        ;;
      --out)
        out="${2:-}"
        shift 2
        ;;
      --sig-out)
        sig_out="${2:-}"
        shift 2
        ;;
      --tag)
        tag="${2:-}"
        shift 2
        ;;
      --commit)
        commit="${2:-}"
        shift 2
        ;;
      --require-ubl-cid)
        require_ubl_cid="true"
        shift
        ;;
      -h|--help)
        cat <<'USAGE'
Usage: scripts/docs_attest.sh sign --manifest <path> --key <private.pem> [--pub <public.pem>] [--out <attestation.json>] [--sig-out <sig.bin>] [--tag <tag>] [--commit <sha>] [--require-ubl-cid]

If key is encrypted, set DOCS_ATTEST_KEY_PASSPHRASE.
Set DOCS_ATTEST_WITH_UBL_CID=true to compute manifest CID via ublx.
USAGE
        return 0
        ;;
      *)
        echo "[error] unknown arg for sign: $1" >&2
        return 1
        ;;
    esac
  done

  [[ -n "$manifest" ]] || { echo "[error] --manifest required" >&2; return 1; }
  [[ -n "$key" ]] || { echo "[error] --key required" >&2; return 1; }
  [[ -f "$manifest" ]] || { echo "[error] manifest not found: $manifest" >&2; return 1; }
  [[ -f "$key" ]] || { echo "[error] key not found: $key" >&2; return 1; }

  mkdir -p "$(dirname "$out")"
  if [[ -z "$sig_out" ]]; then
    sig_out="${out%.json}.sig"
  fi
  mkdir -p "$(dirname "$sig_out")"

  if [[ -z "$commit" ]]; then
    commit="$(git -C "$ROOT_DIR" rev-parse HEAD 2>/dev/null || true)"
  fi
  if [[ -z "$tag" ]]; then
    tag="$(git -C "$ROOT_DIR" describe --tags --exact-match 2>/dev/null || true)"
  fi

  local tmp
  tmp="$(mktemp -d)"

  local pub_path="$pub"
  if [[ -z "$pub_path" ]]; then
    pub_path="$tmp/public.pem"
    if [[ -n "${DOCS_ATTEST_KEY_PASSPHRASE:-}" ]]; then
      openssl pkey -in "$key" -passin "pass:${DOCS_ATTEST_KEY_PASSPHRASE}" -pubout -out "$pub_path"
    else
      openssl pkey -in "$key" -pubout -out "$pub_path"
    fi
  fi
  [[ -f "$pub_path" ]] || { echo "[error] public key not found: $pub_path" >&2; return 1; }

  if [[ -n "${DOCS_ATTEST_KEY_PASSPHRASE:-}" ]]; then
    openssl pkeyutl -sign -rawin -inkey "$key" -passin "pass:${DOCS_ATTEST_KEY_PASSPHRASE}" -in "$manifest" -out "$sig_out"
  else
    openssl pkeyutl -sign -rawin -inkey "$key" -in "$manifest" -out "$sig_out"
  fi

  openssl pkeyutl -verify -rawin -pubin -inkey "$pub_path" -sigfile "$sig_out" -in "$manifest" >/dev/null

  local manifest_sha key_id signature_b64 generated_at
  manifest_sha="$(sha256_file "$manifest")"
  key_id="sha256:$(openssl pkey -pubin -in "$pub_path" -outform DER | sha256_stdin)"
  signature_b64="$(base64 < "$sig_out" | tr -d '\r\n')"
  generated_at="$(now_utc)"

  local manifest_cid=""
  if [[ "$with_ubl_cid" == "true" ]] && command -v cargo >/dev/null 2>&1; then
    manifest_cid="$(cd "$ROOT_DIR" && cargo run -q -p ubl_cli -- cid "$manifest" 2>/dev/null || true)"
  fi
  if [[ "$require_ubl_cid" == "true" && -z "$manifest_cid" ]]; then
    echo "[error] could not compute UBL CID via 'cargo run -p ubl_cli -- cid'" >&2
    return 1
  fi

  local pub_pem
  pub_pem="$(cat "$pub_path")"

  jq -n \
    --arg atype "ubl/docs-attestation" \
    --arg aver "1.0" \
    --arg world "a/logline/t/oss" \
    --arg generated_at "$generated_at" \
    --arg commit "$commit" \
    --arg tag "$tag" \
    --arg manifest_path "$manifest" \
    --arg manifest_sha "$manifest_sha" \
    --arg manifest_cid "$manifest_cid" \
    --arg sig_alg "ed25519" \
    --arg key_id "$key_id" \
    --arg pub_pem "$pub_pem" \
    --arg sig_b64 "$signature_b64" \
    '{
      "@type": $atype,
      "@ver": $aver,
      "@world": $world,
      "generated_at": $generated_at,
      "git": {
        "commit": (if $commit == "" then null else $commit end),
        "tag": (if $tag == "" then null else $tag end)
      },
      "manifest": {
        "path": $manifest_path,
        "sha256": $manifest_sha,
        "cid": (if $manifest_cid == "" then null else $manifest_cid end)
      },
      "signature": {
        "alg": $sig_alg,
        "key_id": $key_id,
        "public_key_pem": $pub_pem,
        "sig_b64": $sig_b64
      }
    }' > "$out"

  printf '%s\n' "$signature_b64" > "${sig_out}.b64"
  rm -rf "$tmp"

  echo "[ok] signature written: $sig_out"
  echo "[ok] signature b64:     ${sig_out}.b64"
  echo "[ok] attestation:       $out"
  echo "[ok] key id:            $key_id"
  if [[ -n "$manifest_cid" ]]; then
    echo "[ok] manifest cid:       $manifest_cid"
  fi
}

cmd_verify() {
  local manifest=""
  local attestation=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --manifest)
        manifest="${2:-}"
        shift 2
        ;;
      --attestation)
        attestation="${2:-}"
        shift 2
        ;;
      -h|--help)
        cat <<'USAGE'
Usage: scripts/docs_attest.sh verify --manifest <path> --attestation <attestation.json>
USAGE
        return 0
        ;;
      *)
        echo "[error] unknown arg for verify: $1" >&2
        return 1
        ;;
    esac
  done

  [[ -n "$manifest" ]] || { echo "[error] --manifest required" >&2; return 1; }
  [[ -n "$attestation" ]] || { echo "[error] --attestation required" >&2; return 1; }
  [[ -f "$manifest" ]] || { echo "[error] manifest not found: $manifest" >&2; return 1; }
  [[ -f "$attestation" ]] || { echo "[error] attestation not found: $attestation" >&2; return 1; }

  local tmp
  tmp="$(mktemp -d)"

  local expected_sha actual_sha sig_b64 pub_pem manifest_cid expected_cid
  expected_sha="$(jq -r '.manifest.sha256 // empty' "$attestation")"
  expected_cid="$(jq -r '.manifest.cid // empty' "$attestation")"
  sig_b64="$(jq -r '.signature.sig_b64 // empty' "$attestation")"
  pub_pem="$(jq -r '.signature.public_key_pem // empty' "$attestation")"

  [[ -n "$expected_sha" ]] || { echo "[error] attestation missing manifest.sha256" >&2; return 1; }
  [[ -n "$sig_b64" ]] || { echo "[error] attestation missing signature.sig_b64" >&2; return 1; }
  [[ -n "$pub_pem" ]] || { echo "[error] attestation missing signature.public_key_pem" >&2; return 1; }

  actual_sha="$(sha256_file "$manifest")"
  if [[ "$actual_sha" != "$expected_sha" ]]; then
    echo "[error] manifest sha mismatch: expected=$expected_sha actual=$actual_sha" >&2
    return 1
  fi

  printf '%s\n' "$pub_pem" > "$tmp/pub.pem"
  printf '%s' "$sig_b64" | base64 --decode > "$tmp/sig.bin" 2>/dev/null || printf '%s' "$sig_b64" | base64 -D > "$tmp/sig.bin"

  openssl pkeyutl -verify -rawin -pubin -inkey "$tmp/pub.pem" -sigfile "$tmp/sig.bin" -in "$manifest" >/dev/null

  if [[ -n "$expected_cid" && "$expected_cid" != "null" ]]; then
    if command -v cargo >/dev/null 2>&1; then
      manifest_cid="$(cd "$ROOT_DIR" && cargo run -q -p ubl_cli -- cid "$manifest" 2>/dev/null || true)"
      if [[ -n "$manifest_cid" && "$manifest_cid" != "$expected_cid" ]]; then
        echo "[error] manifest cid mismatch: expected=$expected_cid actual=$manifest_cid" >&2
        return 1
      fi
      if [[ -n "$manifest_cid" ]]; then
        echo "[ok] cid matches: $manifest_cid"
      fi
    else
      echo "[warn] cargo not available; skipping CID verification"
    fi
  fi

  rm -rf "$tmp"
  echo "[ok] manifest sha matches"
  echo "[ok] signature verifies"
}

main() {
  local cmd="${1:-}"
  if [[ -z "$cmd" ]]; then
    usage
    exit 1
  fi
  shift

  case "$cmd" in
    init-key) cmd_init_key "$@" ;;
    build-manifest) cmd_build_manifest "$@" ;;
    sign) cmd_sign "$@" ;;
    verify) cmd_verify "$@" ;;
    -h|--help|help) usage ;;
    *)
      echo "[error] unknown command: $cmd" >&2
      usage
      exit 1
      ;;
  esac
}

main "$@"
