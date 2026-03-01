#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

OUT_DIR="${ROOT_DIR}/artifacts/contract"
OUT_JSON=""
OUT_MD=""

usage() {
  cat <<'USAGE'
Run UBL-CORE contract suite and emit canonical report artifacts.

Usage:
  scripts/contract_suite.sh [--out-dir <dir>] [--out-json <file>] [--out-md <file>]

Defaults:
  --out-dir  artifacts/contract
  --out-json <out-dir>/latest.json
  --out-md   <out-dir>/latest.md
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      OUT_DIR="${2:-}"
      shift 2
      ;;
    --out-json)
      OUT_JSON="${2:-}"
      shift 2
      ;;
    --out-md)
      OUT_MD="${2:-}"
      shift 2
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

require_cmd() {
  local c="$1"
  command -v "$c" >/dev/null 2>&1 || {
    echo "[error] missing command: $c" >&2
    exit 1
  }
}

require_cmd cargo
require_cmd jq
require_cmd git
require_cmd python3

if [[ -z "$OUT_JSON" ]]; then
  OUT_JSON="${OUT_DIR}/latest.json"
fi
if [[ -z "$OUT_MD" ]]; then
  OUT_MD="${OUT_DIR}/latest.md"
fi

mkdir -p "$OUT_DIR" "$OUT_DIR/logs"

TMP_JSONL="$(mktemp)"
trap 'rm -f "$TMP_JSONL"' EXIT

RUN_ID="contract-$(date -u +%Y%m%dT%H%M%SZ)"
GENERATED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
GIT_SHA="$(git -C "$ROOT_DIR" rev-parse HEAD 2>/dev/null || echo unknown)"

# id|name|command
RULES=$(
  cat <<'EOF'
CONTRACT-001|KNOCK vector matrix|cargo test -p ubl_runtime --test knock_vector_matrix
CONTRACT-002|Rho contract vectors|cargo test -p rb_vm --test rho_contract_vectors
CONTRACT-003|Canon guardrails|cargo test -p ubl_runtime --test canon_guardrails
CONTRACT-004|Gate knock deny receipt contract|cargo test -p ubl_gate chips_endpoint_invalid_json_emits_knock_deny_receipt
CONTRACT-005|Runtime subject_did and knock_cid contract|cargo test -p ubl_runtime process_chip_with_context_sets_subject_and_knock_cid && cargo test -p ubl_runtime knock_rejection_produces_signed_deny_receipt
EOF
)

run_rule() {
  local id="$1"
  local name="$2"
  local cmd="$3"
  local log_file="${OUT_DIR}/logs/${id}.log"
  local t0 t1 duration_ms status exit_code

  echo "[info] ${id} :: ${name}"
  t0="$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)"

  if (cd "$ROOT_DIR" && bash -lc "$cmd") >"$log_file" 2>&1; then
    status="PASS"
    exit_code=0
  else
    status="FAIL"
    exit_code=$?
  fi

  t1="$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)"
  duration_ms=$((t1 - t0))

  jq -nc \
    --arg id "$id" \
    --arg name "$name" \
    --arg command "$cmd" \
    --arg status "$status" \
    --arg log_file "${log_file#$ROOT_DIR/}" \
    --argjson exit_code "$exit_code" \
    --argjson duration_ms "$duration_ms" \
    '{
      id:$id,
      name:$name,
      command:$command,
      status:$status,
      exit_code:$exit_code,
      duration_ms:$duration_ms,
      log_file:$log_file
    }' >> "$TMP_JSONL"

  echo "[${status}] ${id} (${duration_ms} ms)"
}

while IFS='|' read -r id name cmd; do
  [[ -z "$id" ]] && continue
  run_rule "$id" "$name" "$cmd"
done <<< "$RULES"

jq -s \
  --arg run_id "$RUN_ID" \
  --arg generated_at "$GENERATED_AT" \
  --arg git_sha "$GIT_SHA" \
  '{
    suite:"ubl-core-contract",
    version:"v1",
    run_id:$run_id,
    generated_at:$generated_at,
    git_sha:$git_sha,
    rules:.,
    totals:{
      total:length,
      pass:(map(select(.status=="PASS")) | length),
      fail:(map(select(.status=="FAIL")) | length)
    },
    ok:(map(select(.status=="FAIL")) | length == 0)
  }' "$TMP_JSONL" > "$OUT_JSON"

{
  echo "# UBL-CORE Contract Report"
  echo ""
  echo "- run_id: \`${RUN_ID}\`"
  echo "- generated_at: \`${GENERATED_AT}\`"
  echo "- git_sha: \`${GIT_SHA}\`"
  echo ""
  echo "## Totals"
  echo ""
  echo "- total: $(jq -r '.totals.total' "$OUT_JSON")"
  echo "- pass: $(jq -r '.totals.pass' "$OUT_JSON")"
  echo "- fail: $(jq -r '.totals.fail' "$OUT_JSON")"
  echo "- ok: $(jq -r '.ok' "$OUT_JSON")"
  echo ""
  echo "## Rules"
  echo ""
  echo "| Rule | Status | Duration (ms) | Exit |"
  echo "|---|---|---:|---:|"
  jq -r '.rules[] | "| \(.id) - \(.name) | \(.status) | \(.duration_ms) | \(.exit_code) |"' "$OUT_JSON"
  echo ""
  echo "## Logs"
  echo ""
  jq -r '.rules[] | "- \(.id): `\(.log_file)`"' "$OUT_JSON"
} > "$OUT_MD"

echo "[ok] contract report json: $OUT_JSON"
echo "[ok] contract report md:   $OUT_MD"

if [[ "$(jq -r '.ok' "$OUT_JSON")" == "true" ]]; then
  exit 0
fi

echo "[error] contract suite failed. Inspect report/logs."
exit 1
