#!/usr/bin/env bash
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="${FOREVER_ENV_FILE:-${SCRIPT_DIR}/../ops/forever_bootstrap.env}"
DRY_RUN="false"

usage() {
  cat <<USAGE
Safe work-zone cleanup (non-destructive to genesis layer).

Usage:
  scripts/workzone_cleanup.sh [--env <file>] [--dry-run]

Safety:
  - Never deletes state identity files, runtime db, receipts, keys, or release binaries.
  - Only cleans temporary/download/cache material explicitly allowlisted.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --env)
      ENV_FILE="${2:-}"
      shift 2
      ;;
    --dry-run)
      DRY_RUN="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[error] unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

log() {
  local lvl="$1"
  shift
  printf '[%s] %s\n' "$lvl" "$*"
}

run() {
  if [[ "$DRY_RUN" == "true" ]]; then
    log "dry-run" "$*"
    return 0
  fi
  "$@"
}

now_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

sha256_file() {
  local f="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$f" | awk '{print $1}'
  else
    shasum -a 256 "$f" | awk '{print $1}'
  fi
}

if [[ -f "$ENV_FILE" ]]; then
  # shellcheck disable=SC1090
  set -a && source "$ENV_FILE" && set +a
else
  log "warn" "env file not found ($ENV_FILE), using defaults"
fi

UBL_BASE_DIR="${UBL_BASE_DIR:-$HOME/ubl-core-forever}"
UBL_RELEASE_TAG="${UBL_RELEASE_TAG:-}"
UBL_TMP_GLOBS="${UBL_TMP_GLOBS:-/tmp/ubl-*,/tmp/ubl_*,/tmp/vcx_*}"
UBL_ALLOW_NON_TMP_GLOBS="${UBL_ALLOW_NON_TMP_GLOBS:-false}"
UBL_DOWNLOAD_RETENTION_DAYS="${UBL_DOWNLOAD_RETENTION_DAYS:-21}"
UBL_CLEAN_CARGO_CACHE="${UBL_CLEAN_CARGO_CACHE:-false}"
UBL_CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"

STATE_DIR="$UBL_BASE_DIR/state"
LIVE_DIR="$UBL_BASE_DIR/live"
RELEASES_DIR="$UBL_BASE_DIR/releases"
DOWNLOADS_DIR="$STATE_DIR/downloads"
REPORT_DIR="$STATE_DIR/maintenance"
REPORT_TS="$(date -u +%Y%m%dT%H%M%SZ)"
REPORT_FILE="$REPORT_DIR/workzone_cleanup_${REPORT_TS}.json"

mkdir -p "$REPORT_DIR"

candidates_file="$(mktemp)"
warnings_file="$(mktemp)"
trap 'rm -f "$candidates_file" "$warnings_file"' EXIT
: > "$candidates_file"
: > "$warnings_file"

log "info" "phase 1: build cleanup candidate list"
if [[ -d "$DOWNLOADS_DIR" ]]; then
  while IFS= read -r d; do
    [[ -z "$d" ]] && continue
    tag_name="$(basename "$d")"
    if [[ -n "$UBL_RELEASE_TAG" && "$tag_name" == "$UBL_RELEASE_TAG" ]]; then
      continue
    fi
    printf '%s\n' "$d" >> "$candidates_file"
  done < <(find "$DOWNLOADS_DIR" -mindepth 1 -maxdepth 1 -type d -mtime +"$UBL_DOWNLOAD_RETENTION_DAYS" | sort)
fi

if [[ -d "$RELEASES_DIR" ]]; then
  find "$RELEASES_DIR" -type d -name '.tmp-unpack' | sort >> "$candidates_file" || true
fi

IFS=',' read -r -a tmp_globs <<< "$UBL_TMP_GLOBS"
for g in "${tmp_globs[@]}"; do
  if [[ "$g" != /tmp/* && "$UBL_ALLOW_NON_TMP_GLOBS" != "true" ]]; then
    echo "blocked tmp glob outside /tmp (set UBL_ALLOW_NON_TMP_GLOBS=true to allow): $g" >> "$warnings_file"
    continue
  fi
  for p in $g; do
    [[ -e "$p" ]] && printf '%s\n' "$p" >> "$candidates_file"
  done
done

if [[ "$UBL_CLEAN_CARGO_CACHE" == "true" ]]; then
  if [[ -d "$UBL_CARGO_HOME/registry/cache" ]]; then
    printf '%s\n' "$UBL_CARGO_HOME/registry/cache" >> "$candidates_file"
  fi
  if [[ -d "$UBL_CARGO_HOME/git/checkouts" ]]; then
    printf '%s\n' "$UBL_CARGO_HOME/git/checkouts" >> "$candidates_file"
  fi
fi

sort -u "$candidates_file" -o "$candidates_file"

log "info" "phase 2: safety gate (protect genesis layer)"
safe_candidates="$(mktemp)"
trap 'rm -f "$candidates_file" "$warnings_file" "$safe_candidates"' EXIT
: > "$safe_candidates"

while IFS= read -r c; do
  [[ -z "$c" ]] && continue

  case "$c" in
    "$STATE_DIR"|"$LIVE_DIR"|"$RELEASES_DIR"|"$UBL_BASE_DIR")
      echo "blocked candidate (protected root): $c" >> "$warnings_file"
      continue
      ;;
  esac

  if [[ "$c" == "$STATE_DIR"/* ]]; then
    # State layer is protected except downloads older than retention.
    if [[ "$c" != "$DOWNLOADS_DIR"/* ]]; then
      echo "blocked candidate under protected state dir: $c" >> "$warnings_file"
      continue
    fi
  fi

  printf '%s\n' "$c" >> "$safe_candidates"
done < "$candidates_file"

sort -u "$safe_candidates" -o "$safe_candidates"

log "info" "phase 3: secret hygiene checks"
runtime_env="$LIVE_DIR/config/runtime.env"
if [[ -f "$runtime_env" ]]; then
  perm="$(stat -f '%Sp' "$runtime_env" 2>/dev/null || stat -c '%A' "$runtime_env" 2>/dev/null || true)"
  if [[ -n "$perm" ]]; then
    echo "runtime_env_perm=$perm" >> "$warnings_file"
  fi
fi

if [[ -f "$HOME/.bash_history" ]]; then
  if rg -n "SIGNING_KEY_HEX|UBL_STAGE_SECRET" "$HOME/.bash_history" >/dev/null 2>&1; then
    echo "secret pattern found in ~/.bash_history" >> "$warnings_file"
  fi
fi

log "info" "phase 4: apply cleanup"
removed_count=0
while IFS= read -r c; do
  [[ -z "$c" ]] && continue
  if [[ "$DRY_RUN" == "true" ]]; then
    log "dry-run" "rm -rf $c"
  else
    rm -rf "$c"
  fi
  removed_count=$((removed_count + 1))
done < "$safe_candidates"

log "info" "phase 5: emit report"
candidates_json="$(mktemp)"
warnings_json="$(mktemp)"
trap 'rm -f "$candidates_file" "$warnings_file" "$safe_candidates" "$candidates_json" "$warnings_json"' EXIT

if [[ -s "$safe_candidates" ]]; then
  jq -R -s 'split("\n") | map(select(length > 0))' "$safe_candidates" > "$candidates_json"
else
  echo '[]' > "$candidates_json"
fi

if [[ -s "$warnings_file" ]]; then
  jq -R -s 'split("\n") | map(select(length > 0))' "$warnings_file" > "$warnings_json"
else
  echo '[]' > "$warnings_json"
fi

jq -n \
  --arg generated_at "$(now_utc)" \
  --arg base_dir "$UBL_BASE_DIR" \
  --arg state_dir "$STATE_DIR" \
  --arg mode "$( [[ "$DRY_RUN" == "true" ]] && echo dry-run || echo apply )" \
  --argjson removed_count "$removed_count" \
  --slurpfile candidates "$candidates_json" \
  --slurpfile warnings "$warnings_json" \
  '{
    generated_at:$generated_at,
    base_dir:$base_dir,
    state_dir:$state_dir,
    mode:$mode,
    removed_count:$removed_count,
    candidates:$candidates[0],
    warnings:$warnings[0]
  }' > "$REPORT_FILE"

if [[ -f "$REPORT_FILE" ]]; then
  chmod 600 "$REPORT_FILE"
  report_sha="$(sha256_file "$REPORT_FILE")"
  log "ok" "cleanup report: $REPORT_FILE"
  log "ok" "cleanup report sha256: $report_sha"
fi

log "ok" "work-zone cleanup complete"
