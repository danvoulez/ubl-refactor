#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Onboard a ChatGPT connector into UBL and mint a scoped token.

Minimal mode (default):
  - create/resolve connector DID
  - submit ubl/user under a/<app>
  - submit ubl/token under a/<app>/t/<tenant>

Full bootstrap mode (--full-bootstrap):
  - also creates ubl/app, ubl/tenant, ubl/membership
  - requires --founder-signing-key-hex to issue signed @cap capabilities

Examples:
  scripts/onboard_chatgpt_connector.sh \
    --gate https://api.ubl.agency \
    --app chip-registry \
    --tenant logline

  scripts/onboard_chatgpt_connector.sh \
    --full-bootstrap \
    --founder-signing-key-hex "$FOUNDER_SIGNING_KEY_HEX" \
    --app chip-registry --tenant logline
USAGE
}

log() { printf '[%s] %s\n' "$1" "$2"; }
die() { log "error" "$1"; exit 1; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing command: $1"
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

GATE_URL="http://127.0.0.1:4000"
APP_SLUG="chip-registry"
APP_DISPLAY_NAME="Chip Registry and Certified Runtime"
TENANT_SLUG="logline"
TENANT_DISPLAY_NAME="LogLine Trust"
CONNECTOR_NAME="ChatGPT Connector"
CONNECTOR_DID=""
CONNECTOR_SIGNING_KEY_HEX=""
TOKEN_ID=""
TOKEN_SCOPE="read,write,mcp,mcp:write,chip:write"
TOKEN_TTL_DAYS="30"
OUTPUT_DIR=""
API_KEY=""
BEARER_TOKEN=""
FULL_BOOTSTRAP="false"
FOUNDER_SIGNING_KEY_HEX=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --gate) GATE_URL="${2:-}"; shift 2;;
    --app) APP_SLUG="${2:-}"; shift 2;;
    --app-display-name) APP_DISPLAY_NAME="${2:-}"; shift 2;;
    --tenant) TENANT_SLUG="${2:-}"; shift 2;;
    --tenant-display-name) TENANT_DISPLAY_NAME="${2:-}"; shift 2;;
    --connector-name) CONNECTOR_NAME="${2:-}"; shift 2;;
    --connector-did) CONNECTOR_DID="${2:-}"; shift 2;;
    --connector-signing-key-hex) CONNECTOR_SIGNING_KEY_HEX="${2:-}"; shift 2;;
    --token-id) TOKEN_ID="${2:-}"; shift 2;;
    --token-scope) TOKEN_SCOPE="${2:-}"; shift 2;;
    --token-ttl-days) TOKEN_TTL_DAYS="${2:-}"; shift 2;;
    --output-dir) OUTPUT_DIR="${2:-}"; shift 2;;
    --api-key) API_KEY="${2:-}"; shift 2;;
    --bearer-token) BEARER_TOKEN="${2:-}"; shift 2;;
    --full-bootstrap) FULL_BOOTSTRAP="true"; shift;;
    --founder-signing-key-hex) FOUNDER_SIGNING_KEY_HEX="${2:-}"; shift 2;;
    -h|--help) usage; exit 0;;
    *) die "unknown arg: $1";;
  esac
done

require_cmd jq
require_cmd curl
require_cmd date

if [[ -z "${OUTPUT_DIR}" ]]; then
  OUTPUT_DIR="${REPO_DIR}/artifacts/onboarding-chatgpt-$(date -u +%Y%m%dT%H%M%SZ)"
fi
mkdir -p "${OUTPUT_DIR}/chips" "${OUTPUT_DIR}/responses" "${OUTPUT_DIR}/caps"
chmod 700 "${OUTPUT_DIR}" || true

if [[ "${FULL_BOOTSTRAP}" == "true" && -z "${FOUNDER_SIGNING_KEY_HEX}" ]]; then
  die "--full-bootstrap requires --founder-signing-key-hex"
fi

ubx() {
  if command -v ublx >/dev/null 2>&1 && ublx --help 2>/dev/null | grep -qE '^\s+did\b'; then
    ublx "$@"
  else
    (cd "${REPO_DIR}" && cargo run -q -p ubl_cli -- "$@")
  fi
}

declare -a auth_args=()
if [[ -n "${API_KEY}" ]]; then
  auth_args+=(-H "x-api-key: ${API_KEY}")
elif [[ -n "${BEARER_TOKEN}" ]]; then
  auth_args+=(-H "authorization: Bearer ${BEARER_TOKEN}")
fi

date_plus_days_utc() {
  local days="$1"
  if date -u -v+"${days}"d +"%Y-%m-%dT%H:%M:%SZ" >/dev/null 2>&1; then
    date -u -v+"${days}"d +"%Y-%m-%dT%H:%M:%SZ"
  else
    date -u -d "+${days} days" +"%Y-%m-%dT%H:%M:%SZ"
  fi
}

WORLD_APP="a/${APP_SLUG}"
WORLD_TENANT="${WORLD_APP}/t/${TENANT_SLUG}"

if [[ -z "${TOKEN_ID}" ]]; then
  TOKEN_ID="tok-chatgpt-$(date -u +%Y%m%d%H%M%S)"
fi

if [[ -z "${CONNECTOR_DID}" ]]; then
  log "info" "no connector DID provided; generating Ed25519 did:key"
  ubx did generate --strict --output "${OUTPUT_DIR}/connector.did.json"
  chmod 600 "${OUTPUT_DIR}/connector.did.json" || true
  CONNECTOR_DID="$(jq -r '.did' "${OUTPUT_DIR}/connector.did.json")"
  CONNECTOR_SIGNING_KEY_HEX="$(jq -r '.signing_key_hex' "${OUTPUT_DIR}/connector.did.json")"
fi

[[ "${CONNECTOR_DID}" == did:* ]] || die "connector DID must start with did:"

SCOPE_JSON="$(jq -Rn --arg s "${TOKEN_SCOPE}" '$s | split(",") | map(gsub("^\\s+|\\s+$";"")) | map(select(length>0))')"
EXPIRES_AT="$(date_plus_days_utc "${TOKEN_TTL_DAYS}")"
KID="${CONNECTOR_DID}#ed25519"

issue_cap() {
  local action="$1"
  local audience="$2"
  local out="$3"
  ubx cap issue \
    --action "${action}" \
    --audience "${audience}" \
    --signing-key-hex "${FOUNDER_SIGNING_KEY_HEX}" \
    --output "${out}"
}

submit_chip() {
  local label="$1"
  local chip_file="$2"
  local resp_file="${OUTPUT_DIR}/responses/${label}.response.json"
  local local_chip_cid
  local_chip_cid="$(ubx cid "${chip_file}")"
  local status
  local -a curl_cmd=(
    curl -sS -o "${resp_file}" -w '%{http_code}'
    -X POST "${GATE_URL%/}/v1/chips"
    -H 'content-type: application/json'
  )
  if [[ ${#auth_args[@]} -gt 0 ]]; then
    curl_cmd+=("${auth_args[@]}")
  fi
  curl_cmd+=(--data-binary "@${chip_file}")
  status="$("${curl_cmd[@]}")"

  if [[ "${status}" -lt 200 || "${status}" -ge 300 ]]; then
    log "error" "${label} failed (HTTP ${status})"
    if [[ "${label}" == "02.user" && "${FULL_BOOTSTRAP}" != "true" ]]; then
      local err_code err_msg
      err_code="$(jq -r '.code // empty' "${resp_file}" 2>/dev/null || true)"
      err_msg="$(jq -r '.message // empty' "${resp_file}" 2>/dev/null || true)"
      if [[ "${err_code}" == "DEPENDENCY_MISSING" && "${err_msg}" == App*not\ found* ]]; then
        log "error" "app '${APP_SLUG}' is missing; rerun with --full-bootstrap (and founder signing key)"
      fi
    fi
    cat "${resp_file}" >&2
    exit 1
  fi

  local decision
  decision="$(jq -r '.decision // empty' "${resp_file}")"
  if [[ "${decision}" != "Allow" ]]; then
    log "error" "${label} denied by pipeline"
    cat "${resp_file}" >&2
    exit 1
  fi

  local receipt_cid chip_cid gate_chain0
  receipt_cid="$(jq -r '.receipt_cid // empty' "${resp_file}")"
  gate_chain0="$(jq -r '.chain[0] // empty' "${resp_file}")"
  chip_cid="${local_chip_cid}"
  [[ -n "${receipt_cid}" ]] || die "${label} missing receipt_cid"
  [[ -n "${chip_cid}" ]] || die "${label} missing chip cid (chain[0])"
  if [[ -n "${gate_chain0}" && "${gate_chain0}" != "${chip_cid}" ]]; then
    log "warn" "${label} response chain[0] (${gate_chain0}) differs from canonical chip cid (${chip_cid})"
  fi
  log "ok" "${label} -> chip_cid=${chip_cid} receipt_cid=${receipt_cid}"
}

sha_short() {
  printf '%s' "$1" | shasum -a 256 | awk '{print substr($1,1,12)}'
}

CONNECTOR_TAG="$(sha_short "${CONNECTOR_DID}")"
USER_ID="user-${CONNECTOR_TAG}"
TOKEN_CHIP_ID="${TOKEN_ID}"

APP_CID=""
USER_CID=""
TENANT_CID=""
MEMBERSHIP_CID=""
TOKEN_CID=""

if [[ "${FULL_BOOTSTRAP}" == "true" ]]; then
  log "info" "full bootstrap mode enabled"
  issue_cap "registry:init" "${WORLD_APP}" "${OUTPUT_DIR}/caps/cap.registry-init.json"
  issue_cap "membership:grant" "${WORLD_TENANT}" "${OUTPUT_DIR}/caps/cap.membership-grant.json"

  FOUNDER_DID="$(ubx did from-key --strict --signing-key-hex "${FOUNDER_SIGNING_KEY_HEX}" | jq -r '.did')"
  [[ "${FOUNDER_DID}" == did:* ]] || die "failed to derive founder did from key"

  jq -n \
    --arg world "${WORLD_APP}" \
    --arg id "app-${APP_SLUG}" \
    --arg slug "${APP_SLUG}" \
    --arg display "${APP_DISPLAY_NAME}" \
    --arg owner "${FOUNDER_DID}" \
    --slurpfile cap "${OUTPUT_DIR}/caps/cap.registry-init.json" \
    '{
      "@type":"ubl/app","@id":$id,"@ver":"1.0","@world":$world,
      "slug":$slug,"display_name":$display,"owner_did":$owner,"@cap":$cap[0]
    }' > "${OUTPUT_DIR}/chips/01.app.json"
  submit_chip "01.app" "${OUTPUT_DIR}/chips/01.app.json"
  APP_CID="$(ubx cid "${OUTPUT_DIR}/chips/01.app.json")"
fi

if [[ "${FULL_BOOTSTRAP}" == "true" ]]; then
  jq -n \
    --arg world "${WORLD_APP}" \
    --arg id "${USER_ID}" \
    --arg did "${CONNECTOR_DID}" \
    --arg name "${CONNECTOR_NAME}" \
    --slurpfile cap "${OUTPUT_DIR}/caps/cap.registry-init.json" \
    '{
      "@type":"ubl/user","@id":$id,"@ver":"1.0","@world":$world,
      "did":$did,"display_name":$name,
      "@cap":$cap[0]
    }' > "${OUTPUT_DIR}/chips/02.user.json"
else
  jq -n \
    --arg world "${WORLD_APP}" \
    --arg id "${USER_ID}" \
    --arg did "${CONNECTOR_DID}" \
    --arg name "${CONNECTOR_NAME}" \
    '{
      "@type":"ubl/user","@id":$id,"@ver":"1.0","@world":$world,
      "did":$did,"display_name":$name
    }' > "${OUTPUT_DIR}/chips/02.user.json"
fi
submit_chip "02.user" "${OUTPUT_DIR}/chips/02.user.json"
USER_CID="$(ubx cid "${OUTPUT_DIR}/chips/02.user.json")"

if [[ "${FULL_BOOTSTRAP}" == "true" ]]; then
  jq -n \
    --arg world "${WORLD_APP}" \
    --arg id "tenant-${TENANT_SLUG}" \
    --arg slug "${TENANT_SLUG}" \
    --arg display "${TENANT_DISPLAY_NAME}" \
    --arg creator "${USER_CID}" \
    '{
      "@type":"ubl/tenant","@id":$id,"@ver":"1.0","@world":$world,
      "slug":$slug,"display_name":$display,"creator_cid":$creator
    }' > "${OUTPUT_DIR}/chips/03.tenant.json"
  submit_chip "03.tenant" "${OUTPUT_DIR}/chips/03.tenant.json"
  TENANT_CID="$(ubx cid "${OUTPUT_DIR}/chips/03.tenant.json")"

  jq -n \
    --arg world "${WORLD_TENANT}" \
    --arg id "membership-${CONNECTOR_TAG}" \
    --arg user_cid "${USER_CID}" \
    --arg tenant_cid "${TENANT_CID}" \
    --arg role "admin" \
    --slurpfile cap "${OUTPUT_DIR}/caps/cap.membership-grant.json" \
    '{
      "@type":"ubl/membership","@id":$id,"@ver":"1.0","@world":$world,
      "user_cid":$user_cid,"tenant_cid":$tenant_cid,"role":$role,
      "@cap":$cap[0]
    }' > "${OUTPUT_DIR}/chips/04.membership.json"
  submit_chip "04.membership" "${OUTPUT_DIR}/chips/04.membership.json"
  MEMBERSHIP_CID="$(ubx cid "${OUTPUT_DIR}/chips/04.membership.json")"
fi

jq -n \
  --arg world "${WORLD_TENANT}" \
  --arg id "${TOKEN_CHIP_ID}" \
  --arg user_cid "${USER_CID}" \
  --arg expires_at "${EXPIRES_AT}" \
  --arg kid "${KID}" \
  --argjson scope "${SCOPE_JSON}" \
  '{
    "@type":"ubl/token","@id":$id,"@ver":"1.0","@world":$world,
    "user_cid":$user_cid,"scope":$scope,"expires_at":$expires_at,"kid":$kid
  }' > "${OUTPUT_DIR}/chips/05.token.json"
submit_chip "05.token" "${OUTPUT_DIR}/chips/05.token.json"
TOKEN_CID="$(ubx cid "${OUTPUT_DIR}/chips/05.token.json")"

jq -n \
  --arg generated_at "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
  --arg gate "${GATE_URL}" \
  --arg mode "$( [[ "${FULL_BOOTSTRAP}" == "true" ]] && echo "full-bootstrap" || echo "join-existing" )" \
  --arg world_app "${WORLD_APP}" \
  --arg world_tenant "${WORLD_TENANT}" \
  --arg connector_did "${CONNECTOR_DID}" \
  --arg connector_name "${CONNECTOR_NAME}" \
  --arg user_cid "${USER_CID}" \
  --arg token_id "${TOKEN_ID}" \
  --arg token_cid "${TOKEN_CID}" \
  --arg token_expires_at "${EXPIRES_AT}" \
  --arg token_scope "${TOKEN_SCOPE}" \
  --arg app_cid "${APP_CID}" \
  --arg tenant_cid "${TENANT_CID}" \
  --arg membership_cid "${MEMBERSHIP_CID}" \
  '{
    generated_at:$generated_at,
    gate:$gate,
    mode:$mode,
    worlds:{app:$world_app,tenant:$world_tenant},
    connector:{did:$connector_did,name:$connector_name,user_cid:$user_cid},
    token:{
      id:$token_id,
      cid:$token_cid,
      expires_at:$token_expires_at,
      scope_csv:$token_scope,
      auth_header: ("Authorization: Bearer " + $token_id)
    },
    bootstrap:{
      app_cid:$app_cid,
      tenant_cid:$tenant_cid,
      membership_cid:$membership_cid
    }
  }' > "${OUTPUT_DIR}/summary.json"

chmod 600 "${OUTPUT_DIR}/summary.json" || true

log "ok" "onboarding complete: ${OUTPUT_DIR}/summary.json"
cat "${OUTPUT_DIR}/summary.json"
