#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

OUT_DIR="${ROOT_DIR}/artifacts/wasm-conformance"
OUT_JSON=""
OUT_MD=""
STRICT_MODE="${WASM_CONFORMANCE_STRICT:-gate}"
MODE="${WASM_CONFORMANCE_MODE:-contract}"

usage() {
  cat <<'USAGE'
Run WASM conformance seed validation and emit canonical report artifacts.

Usage:
  scripts/wasm_conformance.sh [--out-dir <dir>] [--out-json <file>] [--out-md <file>] [--strict <gate|final>] [--mode <contract|runtime|all>]
USAGE
}

if [[ "${1:-}" == "--_validate_vector" ]]; then
  validate_vector() {
    local path="$1"
    python3 - "$path" <<'PY'
import json
import re
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as f:
    obj = json.load(f)

required = ["id", "version", "kind", "category", "description", "profile", "input", "expected"]
for key in required:
    if key not in obj:
        raise SystemExit(f"missing required key: {key}")

if not re.match(r"^WASM-(POS|NEG)-[0-9]{3}$", obj["id"]):
    raise SystemExit("invalid id format")
if obj["version"] != "v1":
    raise SystemExit("unsupported version")
if obj["kind"] not in ("positive", "negative"):
    raise SystemExit("invalid kind")
if obj["category"] not in ("abi", "verify", "capability", "determinism", "resource", "receipt_binding"):
    raise SystemExit("invalid category")
if obj["profile"] != "deterministic_v1":
    raise SystemExit("invalid profile")
if not isinstance(obj["input"], dict):
    raise SystemExit("input must be object")
if not isinstance(obj["expected"], dict):
    raise SystemExit("expected must be object")
if "decision" not in obj["expected"] or "code" not in obj["expected"]:
    raise SystemExit("expected.decision and expected.code are required")
if obj["expected"]["decision"] not in ("allow", "deny"):
    raise SystemExit("expected.decision invalid")
if obj["kind"] == "positive":
    claims = obj["expected"].get("receipt_claims")
    if not isinstance(claims, list) or len(claims) == 0:
        raise SystemExit("positive vectors must define expected.receipt_claims")
    if not all(isinstance(c, str) and c.strip() for c in claims):
        raise SystemExit("receipt_claims entries must be non-empty strings")

print("ok")
PY
  }
  validate_vector "$2"
  exit 0
fi

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
    --strict)
      STRICT_MODE="${2:-}"
      shift 2
      ;;
    --mode)
      MODE="${2:-}"
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

require_cmd jq
require_cmd python3
require_cmd git
if [[ "$MODE" == "runtime" || "$MODE" == "all" ]]; then
  require_cmd cargo
fi

if [[ -z "$OUT_JSON" ]]; then
  OUT_JSON="${OUT_DIR}/latest.json"
fi
if [[ -z "$OUT_MD" ]]; then
  OUT_MD="${OUT_DIR}/latest.md"
fi

mkdir -p "$OUT_DIR"
TMP_JSONL="$(mktemp)"
trap 'rm -f "$TMP_JSONL"' EXIT

RUN_ID="wasm-conf-$(date -u +%Y%m%dT%H%M%SZ)"
GENERATED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
GIT_SHA="$(git -C "$ROOT_DIR" rev-parse HEAD 2>/dev/null || echo unknown)"
SCHEMA_PATH="${ROOT_DIR}/docs/wasm/conformance/VECTOR_SCHEMA_V1.json"
VECTOR_ROOT="${ROOT_DIR}/docs/wasm/conformance/vectors/v1"

run_rule() {
  local id="$1"
  local name="$2"
  local cmd="$3"
  local status="PASS"
  local exit_code=0
  local detail=""

  if ! detail="$(bash -lc "$cmd" 2>&1)"; then
    status="FAIL"
    exit_code=$?
  fi

  jq -nc \
    --arg id "$id" \
    --arg name "$name" \
    --arg command "$cmd" \
    --arg status "$status" \
    --arg detail "$detail" \
    --argjson exit_code "$exit_code" \
    '{id:$id,name:$name,command:$command,status:$status,exit_code:$exit_code,detail:$detail}' >> "$TMP_JSONL"
}

POS_COUNT=0
NEG_COUNT=0

if [[ "$MODE" == "contract" || "$MODE" == "all" ]]; then
  run_rule "WASM-CONF-001" "Schema file exists" "test -f '$SCHEMA_PATH'"
  run_rule "WASM-CONF-002" "Vector root exists" "test -d '$VECTOR_ROOT'"
  run_rule "WASM-CONF-003" "All vectors parse and satisfy required fields" "for f in \$(find \"$VECTOR_ROOT\" -type f -name '*.vector.json' | sort); do \"$0\" --_validate_vector \"\$f\"; done"

  POS_COUNT="$(find "$VECTOR_ROOT/positive" -type f -name '*.vector.json' | wc -l | tr -d ' ')"
  NEG_COUNT="$(find "$VECTOR_ROOT/negative" -type f -name '*.vector.json' | wc -l | tr -d ' ')"
fi

# Baseline gate target (aligned with release DoD).
POS_MIN=30
NEG_MIN=70

if [[ "$MODE" == "contract" || "$MODE" == "all" ]]; then
  if [[ "$POS_COUNT" -lt "$POS_MIN" ]]; then
    run_rule "WASM-CONF-004" "Positive vector count threshold" "echo 'positive vectors: $POS_COUNT, expected >= $POS_MIN' && exit 1"
  else
    run_rule "WASM-CONF-004" "Positive vector count threshold" "echo 'positive vectors: $POS_COUNT, expected >= $POS_MIN'"
  fi
fi

if [[ "$MODE" == "contract" || "$MODE" == "all" ]]; then
  if [[ "$NEG_COUNT" -lt "$NEG_MIN" ]]; then
    run_rule "WASM-CONF-005" "Negative vector count threshold" "echo 'negative vectors: $NEG_COUNT, expected >= $NEG_MIN' && exit 1"
  else
    run_rule "WASM-CONF-005" "Negative vector count threshold" "echo 'negative vectors: $NEG_COUNT, expected >= $NEG_MIN'"
  fi
fi

if [[ "$MODE" == "runtime" || "$MODE" == "all" ]]; then
  run_rule "WASM-CONF-006" "Runtime execution of vector pack (CHECK/TR path)" "cd '$ROOT_DIR' && cargo test -p ubl_runtime stage_runtime_executes_wasm_conformance_vectors -- --nocapture"
fi

jq -s \
  --arg run_id "$RUN_ID" \
  --arg generated_at "$GENERATED_AT" \
  --arg git_sha "$GIT_SHA" \
  --arg strict_mode "$STRICT_MODE" \
  --arg mode "$MODE" \
  --argjson positive_total "$POS_COUNT" \
  --argjson negative_total "$NEG_COUNT" \
  '{
    suite:"ubl-wasm-conformance",
    version:"v1",
    run_id:$run_id,
    generated_at:$generated_at,
    git_sha:$git_sha,
    strict_mode:$strict_mode,
    mode:$mode,
    positive_total:$positive_total,
    negative_total:$negative_total,
    rules:.,
    totals:{
      total:length,
      pass:(map(select(.status=="PASS"))|length),
      fail:(map(select(.status=="FAIL"))|length)
    },
    ok:(map(select(.status=="FAIL"))|length==0)
  }' "$TMP_JSONL" > "$OUT_JSON"

{
  echo "# UBL WASM Conformance Report"
  echo ""
  echo "- run_id: \`$RUN_ID\`"
  echo "- generated_at: \`$GENERATED_AT\`"
  echo "- git_sha: \`$GIT_SHA\`"
  echo "- strict_mode: \`$STRICT_MODE\`"
  echo "- mode: \`$MODE\`"
  echo ""
  echo "## Vector Totals"
  echo ""
  echo "- positive_total: $POS_COUNT"
  echo "- negative_total: $NEG_COUNT"
  echo ""
  echo "## Rules"
  echo ""
  echo "| Rule | Status | Exit |"
  echo "|---|---|---:|"
  jq -r '.rules[] | "| \(.id) - \(.name) | \(.status) | \(.exit_code) |"' "$OUT_JSON"
} > "$OUT_MD"

echo "[ok] wasm conformance json: $OUT_JSON"
echo "[ok] wasm conformance md:   $OUT_MD"

if [[ "$(jq -r '.ok' "$OUT_JSON")" == "true" ]]; then
  exit 0
fi

echo "[error] wasm conformance failed"
exit 1
