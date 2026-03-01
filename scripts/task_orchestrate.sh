#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

GATE_URL="${GATE_URL:-http://127.0.0.1:4000}"
WORLD="${WORLD:-ubl.platform.test}"
OUT_DIR="${OUT_DIR:-${ROOT_DIR}/artifacts/tasks}"
STATE="${STATE:-open}"
UBLX_BIN="${UBLX_BIN:-ublx}"
USE_CARGO_FALLBACK="${USE_CARGO_FALLBACK:-true}"
API_KEY="${API_KEY:-${UBL_GATE_API_KEY:-${SOURCE_GATE_API_KEY:-}}}"

usage() {
  cat <<'USAGE'
Task orchestration bootstrap (chip-native task lifecycle).

Usage:
  scripts/task_orchestrate.sh [options]

Options:
  --gate <url>         Gate URL (default: http://127.0.0.1:4000)
  --world <world>      @world value (default: ubl.platform.test)
  --state <state>      Task state to submit (default: open)
  --out <dir>          Output directory for payloads/responses (default: artifacts/tasks)
  --api-key <key>      Optional API key for write-protected lanes
  --dry-run            Generate payloads only, do not submit
  -h, --help           Show help

Environment:
  GATE_URL, WORLD, STATE, OUT_DIR, UBLX_BIN, USE_CARGO_FALLBACK, API_KEY,
  UBL_GATE_API_KEY, SOURCE_GATE_API_KEY
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

now_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

safe_id_suffix() {
  date -u +"%Y%m%dT%H%M%SZ"
}

run_submit() {
  local payload_file="$1"
  local response_file="$2"

  if command -v "$UBLX_BIN" >/dev/null 2>&1; then
    local -a cmd
    cmd=("$UBLX_BIN" submit --input "$payload_file" --gate "$GATE_URL" --output "$response_file")
    if [[ -n "$API_KEY" ]]; then
      cmd+=(--api-key "$API_KEY")
    fi
    "${cmd[@]}" >/dev/null
    return 0
  fi

  if [[ "$USE_CARGO_FALLBACK" == "true" ]]; then
    local -a cargo_cmd
    cargo_cmd=(cargo run -q -p ublx -- submit --input "$payload_file" --gate "$GATE_URL" --output "$response_file")
    if [[ -n "$API_KEY" ]]; then
      cargo_cmd+=(--api-key "$API_KEY")
    fi
    (cd "$ROOT_DIR" && "${cargo_cmd[@]}" >/dev/null)
    return 0
  fi

  log "error" "ublx not found and USE_CARGO_FALLBACK=false"
  return 1
}

DRY_RUN="false"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --gate)
      GATE_URL="$2"
      shift 2
      ;;
    --world)
      WORLD="$2"
      shift 2
      ;;
    --state)
      STATE="$2"
      shift 2
      ;;
    --out)
      OUT_DIR="$2"
      shift 2
      ;;
    --api-key)
      API_KEY="$2"
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
      log "error" "unknown argument: $1"
      usage
      exit 1
      ;;
  esac
done

if [[ "$STATE" != "open" && "$STATE" != "blocked" && "$STATE" != "in_progress" && "$STATE" != "done" && "$STATE" != "canceled" ]]; then
  log "error" "invalid --state '$STATE'"
  exit 1
fi

require_cmd jq
mkdir -p "$OUT_DIR/payloads" "$OUT_DIR/responses"

TASK_IDS=("L-01" "L-02" "L-03" "L-04" "L-05")
TASK_TITLES=(
  "Publish NRF-1.1 normative spec"
  "Publish @world semantic spec"
  "Map AI Passport into canon reference"
  "Compose WASM receipt binding with canonical receipt"
  "Harden fs_read scoped capability semantics"
)

stamp="$(safe_id_suffix)"
summary_jsonl="$OUT_DIR/summary.${stamp}.jsonl"
: > "$summary_jsonl"

for i in "${!TASK_IDS[@]}"; do
  task_id="${TASK_IDS[$i]}"
  title="${TASK_TITLES[$i]}"
  event_id="task-${task_id}-${STATE}-${stamp}"

  payload_file="$OUT_DIR/payloads/${event_id}.json"
  response_file="$OUT_DIR/responses/${event_id}.response.json"

  jq -n \
    --arg id "$event_id" \
    --arg world "$WORLD" \
    --arg task_id "$task_id" \
    --arg track "track-2" \
    --arg title "$title" \
    --arg state "$STATE" \
    --arg ts "$(now_utc)" \
    '{
      "@id": $id,
      "@type": "task.lifecycle.event.v1",
      "@ver": "v1",
      "@world": $world,
      "task_id": $task_id,
      "track": $track,
      "title": $title,
      "state": $state,
      "depends_on": [],
      "evidence": [],
      "notes": ("seeded by task_orchestrate.sh at " + $ts),
      "actor": {"did": "did:key:task-orchestrator", "role": "platform"}
    }' > "$payload_file"

  if [[ "$DRY_RUN" == "true" ]]; then
    log "info" "dry-run payload created: $payload_file"
    jq -n --arg task_id "$task_id" --arg payload "$payload_file" '{task_id:$task_id,payload:$payload,submitted:false}' >> "$summary_jsonl"
    continue
  fi

  if run_submit "$payload_file" "$response_file"; then
    receipt_cid="$(jq -r '.receipt_cid // empty' "$response_file" 2>/dev/null || true)"
    log "ok" "$task_id submitted receipt=${receipt_cid:-none}"
    jq -n \
      --arg task_id "$task_id" \
      --arg payload "$payload_file" \
      --arg response "$response_file" \
      --arg receipt_cid "$receipt_cid" \
      '{task_id:$task_id,payload:$payload,response:$response,receipt_cid:$receipt_cid,submitted:true}' >> "$summary_jsonl"
  else
    log "error" "$task_id submit failed"
    jq -n \
      --arg task_id "$task_id" \
      --arg payload "$payload_file" \
      --arg response "$response_file" \
      '{task_id:$task_id,payload:$payload,response:$response,submitted:false}' >> "$summary_jsonl"
  fi
done

summary_file="$OUT_DIR/summary.${stamp}.json"
jq -s '.' "$summary_jsonl" > "$summary_file"
rm -f "$summary_jsonl"

log "info" "summary: $summary_file"
