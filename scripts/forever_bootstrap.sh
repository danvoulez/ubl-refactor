#!/usr/bin/env bash
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

ENV_FILE="${FOREVER_ENV_FILE:-${SCRIPT_DIR}/../ops/forever_bootstrap.env}"
DRY_RUN="false"

usage() {
  cat <<USAGE
Enterprise idempotent bootstrap for UBL forever host (LAB 512).

Usage:
  scripts/forever_bootstrap.sh [--env <file>] [--dry-run]

Examples:
  scripts/forever_bootstrap.sh --env ./ops/forever_bootstrap.env
  scripts/forever_bootstrap.sh --dry-run
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

fetch_json_with_retry() {
  local url="$1"
  local out_file="$2"
  local retries="${3:-8}"
  local sleep_secs="${4:-1}"
  local i
  for i in $(seq 1 "$retries"); do
    if curl -fsS "$url" > "$out_file.tmp"; then
      mv "$out_file.tmp" "$out_file"
      return 0
    fi
    sleep "$sleep_secs"
  done
  rm -f "$out_file.tmp"
  return 1
}

require_cmd() {
  local c="$1"
  command -v "$c" >/dev/null 2>&1 || {
    log "error" "missing command: $c"
    exit 1
  }
}

require_file() {
  local f="$1"
  [[ -f "$f" ]] || {
    log "error" "missing file: $f"
    exit 1
  }
}

sha256_file() {
  local f="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$f" | awk '{print $1}'
  else
    shasum -a 256 "$f" | awk '{print $1}'
  fi
}

sha256_text() {
  local text="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    printf '%s' "$text" | sha256sum | awk '{print $1}'
  else
    printf '%s' "$text" | shasum -a 256 | awk '{print $1}'
  fi
}

env_get() {
  local key="$1"
  grep -E "^${key}=" "$RUNTIME_ENV" 2>/dev/null | tail -n1 | cut -d= -f2-
}

upsert_env() {
  local key="$1"
  local val="$2"
  if [[ "$val" == *$'\n'* ]]; then
    log "error" "refusing to write multiline value for $key"
    exit 1
  fi
  if grep -q "^${key}=" "$RUNTIME_ENV" 2>/dev/null; then
    if [[ "$DRY_RUN" == "true" ]]; then
      log "dry-run" "update $key in runtime env"
    else
      awk -v k="$key" -v v="$val" '
        BEGIN{updated=0}
        {
          if($0 ~ "^"k"="){print k"="v; updated=1}
          else print
        }
        END{if(!updated) print k"="v}
      ' "$RUNTIME_ENV" > "$RUNTIME_ENV.tmp"
      mv "$RUNTIME_ENV.tmp" "$RUNTIME_ENV"
    fi
  else
    if [[ "$DRY_RUN" == "true" ]]; then
      log "dry-run" "append $key to runtime env"
    else
      printf '%s=%s\n' "$key" "$val" >> "$RUNTIME_ENV"
    fi
  fi
}

derive_pub_pem_from_seed_hex() {
  local seed_hex="$1"
  local out_pem="$2"
  local tmp_der
  tmp_der="$(mktemp)"
  # PKCS#8 DER prefix for Ed25519 private key seed (RFC 8410)
  printf '302e020100300506032b657004220420%s' "$seed_hex" | xxd -r -p > "$tmp_der"
  openssl pkey -inform DER -in "$tmp_der" -pubout -out "$out_pem" >/dev/null 2>&1
  rm -f "$tmp_der"
}

get_primary_iface() {
  if [[ "$(uname -s)" == "Darwin" ]]; then
    route -n get default 2>/dev/null | awk '/interface:/{print $2; exit}'
  else
    ip route get 1.1.1.1 2>/dev/null | awk '{for(i=1;i<=NF;i++){if($i=="dev"){print $(i+1); exit}}}'
  fi
}

get_iface_mac() {
  local iface="$1"
  if [[ -z "$iface" ]]; then
    echo ""
    return
  fi
  if [[ "$(uname -s)" == "Darwin" ]]; then
    ifconfig "$iface" 2>/dev/null | awk '/ether /{print $2; exit}'
  else
    cat "/sys/class/net/${iface}/address" 2>/dev/null || true
  fi
}

get_disk_serial_or_id() {
  local candidate
  if [[ "$(uname -s)" == "Darwin" ]]; then
    candidate="$(df "$UBL_BASE_DIR" | awk 'NR==2{print $1}')"
    diskutil info "$candidate" 2>/dev/null | awk -F': *' '
      /Serial Number/ {print $2; exit}
      /Disk \/ Partition UUID/ {print $2; exit}
      /Volume UUID/ {print $2; exit}
    '
  else
    candidate="$(df "$UBL_BASE_DIR" | awk 'NR==2{print $1}')"
    udevadm info --query=property --name "$candidate" 2>/dev/null | awk -F= '/^ID_SERIAL=/{print $2; exit}'
  fi
}

capture_machine_birth() {
  if [[ -f "$MACHINE_BIRTH_FILE" ]]; then
    log "ok" "machine birth already exists: $MACHINE_BIRTH_FILE"
    return
  fi
  if [[ "$DRY_RUN" == "true" ]]; then
    log "dry-run" "would capture machine birth: $MACHINE_BIRTH_FILE"
    return
  fi

  local os_pretty=""
  if [[ -f /etc/os-release ]]; then
    os_pretty="$(awk -F= '/^PRETTY_NAME=/{gsub(/"/,"",$2); print $2; exit}' /etc/os-release 2>/dev/null || true)"
  fi
  if [[ -z "$os_pretty" ]]; then
    os_pretty="$(sw_vers -productVersion 2>/dev/null || uname -srv)"
  fi

  local kernel
  kernel="$(uname -r)"
  local arch
  arch="$(uname -m)"
  local rustc_ver
  rustc_ver="$(rustc --version 2>/dev/null || true)"
  local cargo_ver
  cargo_ver="$(cargo --version 2>/dev/null || true)"
  local openssl_ver
  openssl_ver="$(openssl version 2>/dev/null || true)"
  local cloudflared_ver
  cloudflared_ver="$(cloudflared --version 2>/dev/null | head -n1 || true)"
  local host
  host="$(hostname 2>/dev/null || true)"
  local iface
  iface="$(get_primary_iface)"
  local mac
  mac="$(get_iface_mac "$iface")"
  local disk_serial
  disk_serial="$(get_disk_serial_or_id || true)"

  local os_fingerprint
  os_fingerprint="${os_pretty} | kernel=${kernel} | arch=${arch}"

  jq -n \
    --arg created_at "$(now_utc)" \
    --arg os_pretty "$os_pretty" \
    --arg kernel "$kernel" \
    --arg arch "$arch" \
    --arg os_fingerprint "$os_fingerprint" \
    --arg rustc "$rustc_ver" \
    --arg cargo "$cargo_ver" \
    --arg openssl "$openssl_ver" \
    --arg cloudflared "$cloudflared_ver" \
    --arg hostname "$host" \
    --arg primary_nic "$iface" \
    --arg mac_address "$mac" \
    --arg disk_serial "$disk_serial" \
    '{
      created_at:$created_at,
      os_pretty:$os_pretty,
      kernel:$kernel,
      cpu_arch:$arch,
      os_fingerprint:$os_fingerprint,
      toolchain:{rustc:$rustc,cargo:$cargo,openssl:$openssl,cloudflared:$cloudflared},
      hostname:$hostname,
      primary_nic:$primary_nic,
      mac_address:$mac_address,
      disk_serial:$disk_serial
    }' > "$MACHINE_BIRTH_FILE"

  log "ok" "machine birth captured: $MACHINE_BIRTH_FILE"
}

install_or_replace_cron_line() {
  local marker="$1"
  local schedule="$2"
  local cmdline="$3"
  local normalized_marker

  normalized_marker="$(printf '%s' "$marker" | tr '[:upper:]' '[:lower:]' | tr -c 'a-z0-9._-' '_')"

  if [[ "$(uname -s)" == "Darwin" ]]; then
    local minute hour dom mon dow
    read -r minute hour dom mon dow _extra <<< "$schedule"
    if [[ -z "$minute" || -z "$hour" ]]; then
      log "error" "invalid schedule for $marker: $schedule"
      return 1
    fi
    local plist_dir plist_file label
    plist_dir="$HOME/Library/LaunchAgents"
    label="world.logline.ubl.${normalized_marker}"
    plist_file="$plist_dir/${label}.plist"
    mkdir -p "$plist_dir"
    cat > "$plist_file" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>Label</key>
    <string>${label}</string>
    <key>ProgramArguments</key>
    <array>
      <string>/bin/bash</string>
      <string>-lc</string>
      <string>${cmdline}</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
      <key>Hour</key>
      <integer>${hour}</integer>
      <key>Minute</key>
      <integer>${minute}</integer>
    </dict>
    <key>RunAtLoad</key>
    <false/>
  </dict>
</plist>
PLIST
    if [[ "$DRY_RUN" == "true" ]]; then
      log "dry-run" "install launchd job: $plist_file"
      return 0
    fi
    launchctl unload "$plist_file" >/dev/null 2>&1 || true
    launchctl load "$plist_file"
    log "ok" "launchd schedule installed for $marker ($hour:$minute): $plist_file"
    return 0
  fi

  require_cmd crontab

  local current filtered
  current="$(crontab -l 2>/dev/null || true)"
  filtered="$(printf '%s\n' "$current" | grep -vF "$marker" || true)"

  local newline
  newline="${schedule} ${cmdline} # ${marker}"

  if [[ "$DRY_RUN" == "true" ]]; then
    log "dry-run" "install cron: $newline"
    return 0
  fi

  local tmp_cron
  tmp_cron="$(mktemp)"
  {
    printf '%s\n' "$filtered"
    printf '%s\n' "$newline"
  } > "$tmp_cron"
  # Keep a clean crontab file and avoid streaming via stdin to prevent rare hangs.
  awk 'NF || prev_nf {print} {prev_nf=NF}' "$tmp_cron" > "$tmp_cron.clean"
  mv "$tmp_cron.clean" "$tmp_cron"
  crontab "$tmp_cron"
  rm -f "$tmp_cron"
}

enforce_pinned_hash() {
  local label="$1"
  local actual="$2"
  local expected="$3"
  local allow_unpinned="$4"

  if [[ -z "$expected" ]]; then
    if [[ "$allow_unpinned" == "true" ]]; then
      log "warn" "$label pin is not set; running in unpinned mode"
      return 0
    fi
    log "error" "$label pin is required but not set"
    exit 1
  fi

  if [[ "$actual" != "$expected" ]]; then
    log "error" "$label mismatch expected=$expected actual=$actual"
    exit 1
  fi
  log "ok" "$label pin verified"
}

write_redacted_runtime_env() {
  local src_file="$1"
  local dst_file="$2"
  awk -F= '
    BEGIN {
      redacted["SIGNING_KEY_HEX"]=1
      redacted["UBL_STAGE_SECRET"]=1
      redacted["CLOUDFLARE_TUNNEL_TOKEN"]=1
    }
    /^[A-Za-z_][A-Za-z0-9_]*=/ {
      key=$1
      if (key in redacted) {
        print key "=[REDACTED]"
      } else {
        print $0
      }
      next
    }
    { print $0 }
  ' "$src_file" > "$dst_file"
}

generate_receipt_qr_artifacts() {
  local receipt_url="$1"
  local png_file="$2"
  local svg_file="$3"
  local formats have_qrencode fmt trimmed

  if [[ "${UBL_RECEIPT_QR_ENABLE:-true}" != "true" ]]; then
    return 0
  fi

  have_qrencode="false"
  if command -v qrencode >/dev/null 2>&1; then
    have_qrencode="true"
  fi
  if [[ "$have_qrencode" != "true" ]]; then
    if [[ "${UBL_RECEIPT_QR_REQUIRE_TOOL:-false}" == "true" ]]; then
      log "error" "QR generation requested but qrencode is missing"
      return 1
    fi
    log "warn" "qrencode not available; skipping QR artifacts"
    return 0
  fi

  formats="${UBL_RECEIPT_QR_FORMATS:-png,svg}"
  IFS=',' read -r -a fmt <<< "$formats"
  for trimmed in "${fmt[@]}"; do
    trimmed="$(printf '%s' "$trimmed" | xargs)"
    case "$trimmed" in
      png)
        qrencode -l "${UBL_RECEIPT_QR_ECLEVEL:-M}" -s "${UBL_RECEIPT_QR_SCALE:-6}" -t PNG -o "$png_file" "$receipt_url"
        chmod 600 "$png_file"
        ;;
      svg)
        qrencode -l "${UBL_RECEIPT_QR_ECLEVEL:-M}" -t SVG -o "$svg_file" "$receipt_url"
        chmod 600 "$svg_file"
        ;;
      "")
        ;;
      *)
        log "warn" "unsupported QR format '$trimmed' in UBL_RECEIPT_QR_FORMATS"
        ;;
    esac
  done
}

upsert_cloudflare_cname_record() {
  local cf_api="$1"
  local api_token="$2"
  local api_email="$3"
  local global_api_key="$4"
  local zone_id="$5"
  local record_name="$6"
  local target="$7"
  local existing_json rec_count rec_id
  local -a auth_headers

  if [[ -n "$api_email" && -n "$global_api_key" ]]; then
    auth_headers=(-H "X-Auth-Email: ${api_email}" -H "X-Auth-Key: ${global_api_key}")
  elif [[ -n "$api_token" ]]; then
    auth_headers=(-H "Authorization: Bearer ${api_token}")
  else
    log "error" "cloudflare auth is missing for zone ${zone_id} (${record_name})"
    return 1
  fi

  existing_json="$(curl -fsS "${auth_headers[@]}" -H "Content-Type: application/json" \
    "${cf_api}/zones/${zone_id}/dns_records?type=CNAME&name=${record_name}")"
  rec_count="$(echo "$existing_json" | jq -r '.result | length')"

  if [[ "$rec_count" == "0" ]]; then
    run curl -fsS -X POST "${auth_headers[@]}" -H "Content-Type: application/json" \
      "${cf_api}/zones/${zone_id}/dns_records" \
      --data "{\"type\":\"CNAME\",\"name\":\"${record_name}\",\"content\":\"${target}\",\"proxied\":true}" >/dev/null
  else
    rec_id="$(echo "$existing_json" | jq -r '.result[0].id')"
    run curl -fsS -X PUT "${auth_headers[@]}" -H "Content-Type: application/json" \
      "${cf_api}/zones/${zone_id}/dns_records/${rec_id}" \
      --data "{\"type\":\"CNAME\",\"name\":\"${record_name}\",\"content\":\"${target}\",\"proxied\":true}" >/dev/null
  fi
  log "ok" "cloudflare DNS upsert complete for ${record_name} -> ${target}"
}

emit_bootstrap_artifact_chip() {
  local artifact_name="$1"
  local artifact_file="$2"
  local artifact_sha existing_sha existing_receipt artifact_rel chip_id
  local payload_file response_file artifact_receipt_file artifact_receipt_cid
  local artifact_size_bytes artifact_payload_mode

  if [[ ! -f "$artifact_file" ]]; then
    log "warn" "artifact file missing, skipping chip emission: $artifact_name ($artifact_file)"
    return 0
  fi
  if ! jq -e . "$artifact_file" >/dev/null 2>&1; then
    log "error" "artifact file is not valid JSON: $artifact_file"
    return 1
  fi

  artifact_sha="$(sha256_file "$artifact_file")"
  existing_sha=""
  existing_receipt=""
  if [[ -f "$BOOTSTRAP_ARTIFACT_LEDGER_FILE" ]]; then
    existing_sha="$(jq -r --arg name "$artifact_name" '.artifacts[$name].sha256 // empty' "$BOOTSTRAP_ARTIFACT_LEDGER_FILE")"
    existing_receipt="$(jq -r --arg name "$artifact_name" '.artifacts[$name].receipt_cid // empty' "$BOOTSTRAP_ARTIFACT_LEDGER_FILE")"
  fi
  if [[ -n "$existing_sha" && "$existing_sha" == "$artifact_sha" && -n "$existing_receipt" ]]; then
    log "ok" "artifact chip already emitted: $artifact_name ($existing_receipt)"
    return 0
  fi

  artifact_rel="${artifact_file#$UBL_BASE_DIR/}"
  if [[ "$artifact_rel" == "$artifact_file" ]]; then
    artifact_rel="$artifact_file"
  fi

  chip_id="${UBL_BOOTSTRAP_ID}:artifact:${artifact_name}:${artifact_sha:0:16}"
  payload_file="$BOOTSTRAP_DIR/artifact_${artifact_name}.chip.json"
  response_file="$BOOTSTRAP_DIR/artifact_${artifact_name}.response.json"
  artifact_receipt_file="$BOOTSTRAP_DIR/artifact_${artifact_name}.receipt.json"
  artifact_size_bytes="$(wc -c < "$artifact_file" | tr -d '[:space:]')"
  artifact_payload_mode="full"
  if [[ "${artifact_size_bytes:-0}" -gt 65536 ]]; then
    artifact_payload_mode="summary"
  fi

  if [[ "$artifact_payload_mode" == "full" ]]; then
    jq -n \
      --arg id "$chip_id" \
      --arg world "$UBL_BOOTSTRAP_WORLD" \
      --arg generated_at "$(now_utc)" \
      --arg artifact_name "$artifact_name" \
      --arg artifact_path "$artifact_rel" \
      --arg artifact_sha256 "$artifact_sha" \
      --arg api_domain "$UBL_API_DOMAIN" \
      --arg rich_url_domain "$UBL_RICH_URL_DOMAIN" \
      --arg payload_mode "$artifact_payload_mode" \
      --argjson artifact_size_bytes "${artifact_size_bytes:-0}" \
      --slurpfile artifact "$artifact_file" \
      '{
        "@type":"ubl/document",
        "@id":$id,
        "@ver":"1.0",
        "@world":$world,
        "doc_type":"ubl/bootstrap.artifact.v1",
        "generated_at":$generated_at,
        "artifact":{
          "name":$artifact_name,
          "path":$artifact_path,
          "sha256":$artifact_sha256,
          "size_bytes":$artifact_size_bytes,
          "payload_mode":$payload_mode,
          "json":$artifact[0]
        },
        "domains":{
          "api":$api_domain,
          "rich_url":$rich_url_domain
        }
      }' > "$payload_file"
  else
    jq -n \
      --arg id "$chip_id" \
      --arg world "$UBL_BOOTSTRAP_WORLD" \
      --arg generated_at "$(now_utc)" \
      --arg artifact_name "$artifact_name" \
      --arg artifact_path "$artifact_rel" \
      --arg artifact_sha256 "$artifact_sha" \
      --arg api_domain "$UBL_API_DOMAIN" \
      --arg rich_url_domain "$UBL_RICH_URL_DOMAIN" \
      --arg payload_mode "$artifact_payload_mode" \
      --argjson artifact_size_bytes "${artifact_size_bytes:-0}" \
      '{
        "@type":"ubl/document",
        "@id":$id,
        "@ver":"1.0",
        "@world":$world,
        "doc_type":"ubl/bootstrap.artifact.v1",
        "generated_at":$generated_at,
        "artifact":{
          "name":$artifact_name,
          "path":$artifact_path,
          "sha256":$artifact_sha256,
          "size_bytes":$artifact_size_bytes,
          "payload_mode":$payload_mode,
          "json_ref":{"inline":false,"reason":"artifact_too_large_for_inline"}
        },
        "domains":{
          "api":$api_domain,
          "rich_url":$rich_url_domain
        }
      }' > "$payload_file"
  fi

  if ! curl -fsS -X POST "http://127.0.0.1:4000/v1/chips" \
    -H "content-type: application/json" \
    --data-binary "@$payload_file" > "$response_file"; then
    if [[ "$artifact_payload_mode" == "full" ]]; then
      # Retry once with metadata-only payload for endpoints with tighter body limits.
      artifact_payload_mode="summary"
      jq -n \
        --arg id "$chip_id" \
        --arg world "$UBL_BOOTSTRAP_WORLD" \
        --arg generated_at "$(now_utc)" \
        --arg artifact_name "$artifact_name" \
        --arg artifact_path "$artifact_rel" \
        --arg artifact_sha256 "$artifact_sha" \
        --arg api_domain "$UBL_API_DOMAIN" \
        --arg rich_url_domain "$UBL_RICH_URL_DOMAIN" \
        --arg payload_mode "$artifact_payload_mode" \
        --argjson artifact_size_bytes "${artifact_size_bytes:-0}" \
        '{
          "@type":"ubl/document",
          "@id":$id,
          "@ver":"1.0",
          "@world":$world,
          "doc_type":"ubl/bootstrap.artifact.v1",
          "generated_at":$generated_at,
          "artifact":{
            "name":$artifact_name,
            "path":$artifact_path,
            "sha256":$artifact_sha256,
            "size_bytes":$artifact_size_bytes,
            "payload_mode":$payload_mode,
            "json_ref":{"inline":false,"reason":"submit_retry_summary_fallback"}
          },
          "domains":{
            "api":$api_domain,
            "rich_url":$rich_url_domain
          }
        }' > "$payload_file"
      if ! curl -fsS -X POST "http://127.0.0.1:4000/v1/chips" \
        -H "content-type: application/json" \
        --data-binary "@$payload_file" > "$response_file"; then
        log "error" "artifact chip submit failed: $artifact_name"
        return 1
      fi
      log "warn" "artifact chip submit required summary fallback: $artifact_name"
    else
      log "error" "artifact chip submit failed: $artifact_name"
      return 1
    fi
  fi

  artifact_receipt_cid="$(jq -r '.receipt_cid // empty' "$response_file")"
  if [[ -z "$artifact_receipt_cid" ]]; then
    log "error" "artifact chip response missing receipt_cid: $artifact_name"
    return 1
  fi

  if ! fetch_json_with_retry "http://127.0.0.1:4000/v1/receipts/${artifact_receipt_cid}" "$artifact_receipt_file" 10 1; then
    if jq -e '.receipt != null' "$response_file" >/dev/null 2>&1; then
      jq '.receipt' "$response_file" > "$artifact_receipt_file"
      log "warn" "artifact receipt endpoint unavailable; used embedded receipt for $artifact_name"
    else
      log "error" "failed to fetch artifact receipt and response has no embedded receipt: $artifact_name"
      return 1
    fi
  fi

  if [[ ! -f "$BOOTSTRAP_ARTIFACT_LEDGER_FILE" ]]; then
    jq -n '{generated_at:"",artifacts:{}}' > "$BOOTSTRAP_ARTIFACT_LEDGER_FILE"
  fi

  jq \
    --arg generated_at "$(now_utc)" \
    --arg name "$artifact_name" \
    --arg sha256 "$artifact_sha" \
    --arg artifact_path "$artifact_rel" \
    --arg chip_id "$chip_id" \
    --arg receipt_cid "$artifact_receipt_cid" \
    --arg payload_file "$payload_file" \
    --arg receipt_file "$artifact_receipt_file" \
    '.generated_at=$generated_at
     | .artifacts=(.artifacts // {})
     | .artifacts[$name]={
         sha256:$sha256,
         artifact_path:$artifact_path,
         chip_id:$chip_id,
         receipt_cid:$receipt_cid,
         payload_file:$payload_file,
         receipt_file:$receipt_file
       }' \
    "$BOOTSTRAP_ARTIFACT_LEDGER_FILE" > "$BOOTSTRAP_ARTIFACT_LEDGER_FILE.tmp"
  mv "$BOOTSTRAP_ARTIFACT_LEDGER_FILE.tmp" "$BOOTSTRAP_ARTIFACT_LEDGER_FILE"
  chmod 600 "$BOOTSTRAP_ARTIFACT_LEDGER_FILE"
  log "ok" "artifact chip emitted: $artifact_name -> $artifact_receipt_cid"
}

emit_bootstrap_artifact_chips() {
  if [[ "$UBL_EMIT_BOOTSTRAP_ARTIFACT_CHIPS" != "true" ]]; then
    log "info" "bootstrap artifact chips disabled (UBL_EMIT_BOOTSTRAP_ARTIFACT_CHIPS=false)"
    return 0
  fi

  local failed="false"
  local artifact_specs=(
    "machine_birth|$MACHINE_BIRTH_FILE"
    "key_birth|$KEY_BIRTH_FILE"
    "release_provenance|$RELEASE_PROVENANCE_FILE"
    "bootstrap_receipt|$BOOTSTRAP_DIR/receipt.json"
    "bootstrap_trace|$BOOTSTRAP_DIR/trace.json"
    "bootstrap_narrate|$BOOTSTRAP_DIR/narrate.json"
    "bootstrap_witness|$BOOTSTRAP_DIR/witness.json"
    "genesis_symbol|$SYMBOL_FILE"
    "freeze_manifest|$FREEZE_MANIFEST_FILE"
  )
  local spec artifact_name artifact_file

  for spec in "${artifact_specs[@]}"; do
    artifact_name="${spec%%|*}"
    artifact_file="${spec#*|}"
    if ! emit_bootstrap_artifact_chip "$artifact_name" "$artifact_file"; then
      failed="true"
    fi
  done

  if [[ "$failed" == "true" ]]; then
    if [[ "$UBL_REQUIRE_BOOTSTRAP_ARTIFACT_CHIPS" == "true" ]]; then
      log "error" "bootstrap artifact chip emission failed and is required"
      exit 1
    fi
    log "warn" "bootstrap artifact chip emission had failures"
  fi
}

append_local_check() {
  local out_file="$1"
  local name="$2"
  local status="$3"
  local detail="$4"
  jq -nc \
    --arg name "$name" \
    --arg status "$status" \
    --arg detail "$detail" \
    --arg checked_at "$(now_utc)" \
    '{name:$name,status:$status,detail:$detail,checked_at:$checked_at}' >> "$out_file"
}

run_local_bootstrap_checks() {
  local out_file="$1"
  local tmp_jsonl
  local fail_count=0
  local warn_count=0
  tmp_jsonl="$(mktemp)"
  : > "$tmp_jsonl"

  local spec name file_path
  local required_json_specs=(
    "machine_birth|$MACHINE_BIRTH_FILE"
    "key_birth|$KEY_BIRTH_FILE"
    "release_provenance|$RELEASE_PROVENANCE_FILE"
    "bootstrap_receipt|$BOOTSTRAP_DIR/receipt.json"
    "bootstrap_trace|$BOOTSTRAP_DIR/trace.json"
    "bootstrap_narrate|$BOOTSTRAP_DIR/narrate.json"
    "bootstrap_witness|$BOOTSTRAP_DIR/witness.json"
    "genesis_symbol|$SYMBOL_FILE"
    "freeze_manifest|$FREEZE_MANIFEST_FILE"
  )

  for spec in "${required_json_specs[@]}"; do
    name="${spec%%|*}"
    file_path="${spec#*|}"
    if [[ ! -f "$file_path" ]]; then
      append_local_check "$tmp_jsonl" "$name" "fail" "missing file: $file_path"
      fail_count=$((fail_count + 1))
      continue
    fi
    if jq -e . "$file_path" >/dev/null 2>&1; then
      append_local_check "$tmp_jsonl" "$name" "pass" "valid json"
    else
      append_local_check "$tmp_jsonl" "$name" "fail" "invalid json: $file_path"
      fail_count=$((fail_count + 1))
    fi
  done

  if [[ -f "$GENESIS_PUB_PEM" && -f "$GENESIS_PUB_SHA_FILE" ]]; then
    expected_pub_sha="$(tr -d '[:space:]' < "$GENESIS_PUB_SHA_FILE" 2>/dev/null || true)"
    actual_pub_sha="$(sha256_file "$GENESIS_PUB_PEM")"
    if [[ -n "$expected_pub_sha" && "$expected_pub_sha" == "$actual_pub_sha" ]]; then
      append_local_check "$tmp_jsonl" "genesis_pubkey_sha256" "pass" "matches sha file"
    else
      append_local_check "$tmp_jsonl" "genesis_pubkey_sha256" "fail" "sha mismatch expected=$expected_pub_sha actual=$actual_pub_sha"
      fail_count=$((fail_count + 1))
    fi
  else
    append_local_check "$tmp_jsonl" "genesis_pubkey_sha256" "fail" "missing pubkey or sha file"
    fail_count=$((fail_count + 1))
  fi

  if [[ "$UBL_EMIT_BOOTSTRAP_ARTIFACT_CHIPS" == "true" ]]; then
    if [[ -f "$BOOTSTRAP_ARTIFACT_LEDGER_FILE" ]] && jq -e '.artifacts | type == "object"' "$BOOTSTRAP_ARTIFACT_LEDGER_FILE" >/dev/null 2>&1; then
      artifact_count="$(jq -r '.artifacts | length' "$BOOTSTRAP_ARTIFACT_LEDGER_FILE")"
      append_local_check "$tmp_jsonl" "artifact_chip_ledger" "pass" "present with $artifact_count entries"
    else
      append_local_check "$tmp_jsonl" "artifact_chip_ledger" "fail" "missing or invalid: $BOOTSTRAP_ARTIFACT_LEDGER_FILE"
      fail_count=$((fail_count + 1))
    fi
  else
    append_local_check "$tmp_jsonl" "artifact_chip_ledger" "warn" "artifact chip emission disabled"
    warn_count=$((warn_count + 1))
  fi

  if curl -fsS "http://127.0.0.1:4000/healthz" >/dev/null 2>&1; then
    append_local_check "$tmp_jsonl" "local_gate_healthz" "pass" "http://127.0.0.1:4000/healthz"
  else
    append_local_check "$tmp_jsonl" "local_gate_healthz" "fail" "localhost healthz failed"
    fail_count=$((fail_count + 1))
  fi

  if [[ "$UBL_CLOUDFLARE_ENABLE" == "true" ]]; then
    if curl -fsS "https://${UBL_API_DOMAIN}/healthz" >/dev/null 2>&1; then
      append_local_check "$tmp_jsonl" "public_api_healthz" "pass" "https://${UBL_API_DOMAIN}/healthz"
    else
      append_local_check "$tmp_jsonl" "public_api_healthz" "warn" "public api healthz not reachable yet"
      warn_count=$((warn_count + 1))
    fi
  else
    append_local_check "$tmp_jsonl" "public_api_healthz" "warn" "cloudflare tunnel disabled"
    warn_count=$((warn_count + 1))
  fi

  jq -n \
    --arg generated_at "$(now_utc)" \
    --arg api_domain "$UBL_API_DOMAIN" \
    --arg rich_url_domain "$UBL_RICH_URL_DOMAIN" \
    --slurpfile checks "$tmp_jsonl" \
    '{
      generated_at:$generated_at,
      domains:{api:$api_domain,rich_url:$rich_url_domain},
      checks:$checks,
      summary:{
        total:($checks | length),
        pass:(($checks | map(select(.status=="pass"))) | length),
        warn:(($checks | map(select(.status=="warn"))) | length),
        fail:(($checks | map(select(.status=="fail"))) | length)
      }
    }' > "$out_file"
  rm -f "$tmp_jsonl"
  chmod 600 "$out_file"

  LOCAL_CHECKS_FAIL_COUNT="$(jq -r '.summary.fail' "$out_file")"
  LOCAL_CHECKS_WARN_COUNT="$(jq -r '.summary.warn' "$out_file")"
  LOCAL_CHECKS_PASS_COUNT="$(jq -r '.summary.pass' "$out_file")"

  if [[ "$LOCAL_CHECKS_FAIL_COUNT" != "0" ]]; then
    if [[ "$UBL_FINAL_CHECKS_REQUIRE_PASS" == "true" ]]; then
      log "error" "local checks failed: $LOCAL_CHECKS_FAIL_COUNT failing checks (strict mode)"
      exit 1
    fi
    log "warn" "local checks have failures: $LOCAL_CHECKS_FAIL_COUNT"
  fi
}

copy_file_into_bundle_stage() {
  local src="$1"
  local dst_rel="$2"
  if [[ ! -f "$src" ]]; then
    return 0
  fi
  mkdir -p "$(dirname "$FINAL_BUNDLE_STAGE_DIR/$dst_rel")"
  cp -f "$src" "$FINAL_BUNDLE_STAGE_DIR/$dst_rel"
}

copy_dir_into_bundle_stage() {
  local src="$1"
  local dst_rel="$2"
  if [[ ! -d "$src" ]]; then
    return 0
  fi
  mkdir -p "$(dirname "$FINAL_BUNDLE_STAGE_DIR/$dst_rel")"
  rm -rf "$FINAL_BUNDLE_STAGE_DIR/$dst_rel"
  cp -a "$src" "$FINAL_BUNDLE_STAGE_DIR/$dst_rel"
}

build_final_evidence_bundle() {
  if [[ "$UBL_FINAL_BUNDLE_ENABLE" != "true" ]]; then
    log "info" "final evidence bundle disabled (UBL_FINAL_BUNDLE_ENABLE=false)"
    return 0
  fi

  FINAL_BUNDLE_TS="$(date -u +%Y%m%dT%H%M%SZ)"
  FINAL_BUNDLE_NAME="${UBL_FINAL_BUNDLE_BASENAME}-${FINAL_BUNDLE_TS}"
  FINAL_BUNDLE_STAGE_DIR="$UBL_FINAL_BUNDLE_DIR/${FINAL_BUNDLE_NAME}.stage"
  FINAL_BUNDLE_PATH="$UBL_FINAL_BUNDLE_DIR/${FINAL_BUNDLE_NAME}.tar.gz"
  FINAL_BUNDLE_MANIFEST_FILE="$UBL_FINAL_BUNDLE_DIR/${FINAL_BUNDLE_NAME}.manifest.json"
  FINAL_BUNDLE_METADATA_FILE="$UBL_FINAL_BUNDLE_DIR/${FINAL_BUNDLE_NAME}.metadata.json"
  FINAL_BUNDLE_LOCAL_CHECKS_FILE="$UBL_FINAL_BUNDLE_DIR/${FINAL_BUNDLE_NAME}.local_checks.json"
  FINAL_BUNDLE_REPORT_FILE="$UBL_FINAL_BUNDLE_DIR/${FINAL_BUNDLE_NAME}.report.json"

  mkdir -p "$UBL_FINAL_BUNDLE_DIR"
  rm -rf "$FINAL_BUNDLE_STAGE_DIR"
  mkdir -p "$FINAL_BUNDLE_STAGE_DIR"

  copy_file_into_bundle_stage "$MACHINE_BIRTH_FILE" "state/machine_birth.json"
  copy_file_into_bundle_stage "$KEY_BIRTH_FILE" "state/key_birth.json"
  copy_file_into_bundle_stage "$RELEASE_PROVENANCE_FILE" "state/release_provenance.json"
  copy_file_into_bundle_stage "$GENESIS_PUB_PEM" "state/genesis_signer.pub.pem"
  copy_file_into_bundle_stage "$GENESIS_PUB_SHA_FILE" "state/genesis_signer.pub.pem.sha256"
  copy_file_into_bundle_stage "$SYMBOL_FILE" "state/genesis_symbol.json"
  copy_file_into_bundle_stage "$FREEZE_MANIFEST_FILE" "state/freeze_manifest.json"
  copy_file_into_bundle_stage "$BOOTSTRAP_ARTIFACT_LEDGER_FILE" "state/bootstrap/artifact_chips.json"
  copy_dir_into_bundle_stage "$BOOTSTRAP_DIR" "state/bootstrap"
  copy_dir_into_bundle_stage "$RELEASE_DIR/attestation" "release/attestation"
  copy_file_into_bundle_stage "$RELEASE_DIR/bin/ubl_gate.sha256" "release/ubl_gate.sha256"
  copy_file_into_bundle_stage "$CLOUDFLARE_RATE_LIMIT_FILE" "state/cloudflare_rate_limit.json"
  copy_file_into_bundle_stage "$LOCAL_CHECKS_FILE" "state/final/local_checks.json"

  if [[ -f "$RUNTIME_ENV" ]]; then
    mkdir -p "$FINAL_BUNDLE_STAGE_DIR/live/config"
    write_redacted_runtime_env "$RUNTIME_ENV" "$FINAL_BUNDLE_STAGE_DIR/live/config/runtime.public.env"
    chmod 600 "$FINAL_BUNDLE_STAGE_DIR/live/config/runtime.public.env"
  fi

  if [[ "$UBL_FINAL_BUNDLE_INCLUDE_SNAPSHOTS" == "true" && -d "$SNAPSHOT_DIR_ROOT" ]]; then
    latest_snapshot="$(find "$SNAPSHOT_DIR_ROOT" -mindepth 1 -maxdepth 1 -type d | sort | tail -n1 || true)"
    if [[ -n "$latest_snapshot" ]]; then
      copy_dir_into_bundle_stage "$latest_snapshot" "state/snapshots/latest"
    fi
  fi

  files_list="$(mktemp)"
  manifest_jsonl="$(mktemp)"
  find "$FINAL_BUNDLE_STAGE_DIR" -type f | sort > "$files_list"
  : > "$manifest_jsonl"
  while IFS= read -r f; do
    rel="${f#$FINAL_BUNDLE_STAGE_DIR/}"
    jq -nc \
      --arg path "$rel" \
      --arg sha256 "$(sha256_file "$f")" \
      --arg bytes "$(wc -c < "$f" | tr -d '[:space:]')" \
      '{path:$path,sha256:$sha256,bytes:($bytes|tonumber)}' >> "$manifest_jsonl"
  done < "$files_list"
  jq -s '.' "$manifest_jsonl" > "$FINAL_BUNDLE_MANIFEST_FILE"
  rm -f "$files_list" "$manifest_jsonl"

  tar -czf "$FINAL_BUNDLE_PATH" -C "$FINAL_BUNDLE_STAGE_DIR" .
  FINAL_BUNDLE_SHA256="$(sha256_file "$FINAL_BUNDLE_PATH")"
  FINAL_BUNDLE_SIZE_BYTES="$(wc -c < "$FINAL_BUNDLE_PATH" | tr -d '[:space:]')"
  FINAL_BUNDLE_MANIFEST_SHA256="$(sha256_file "$FINAL_BUNDLE_MANIFEST_FILE")"

  jq -n \
    --arg generated_at "$(now_utc)" \
    --arg release_tag "$UBL_RELEASE_TAG" \
    --arg release_commit "$release_commit" \
    --arg api_domain "$UBL_API_DOMAIN" \
    --arg rich_url_domain "$UBL_RICH_URL_DOMAIN" \
    --arg bundle_name "$FINAL_BUNDLE_NAME" \
    --arg bundle_path "$FINAL_BUNDLE_PATH" \
    --arg bundle_sha256 "$FINAL_BUNDLE_SHA256" \
    --arg bundle_size_bytes "$FINAL_BUNDLE_SIZE_BYTES" \
    --arg manifest_path "$FINAL_BUNDLE_MANIFEST_FILE" \
    --arg manifest_sha256 "$FINAL_BUNDLE_MANIFEST_SHA256" \
    --arg local_checks_path "$LOCAL_CHECKS_FILE" \
    --arg include_snapshots "$UBL_FINAL_BUNDLE_INCLUDE_SNAPSHOTS" \
    --slurpfile local_checks "$LOCAL_CHECKS_FILE" \
    '{
      generated_at:$generated_at,
      release:{tag:$release_tag,commit:$release_commit},
      domains:{api:$api_domain,rich_url:$rich_url_domain},
      bundle:{
        name:$bundle_name,
        path:$bundle_path,
        sha256:$bundle_sha256,
        size_bytes:($bundle_size_bytes|tonumber),
        include_snapshots:($include_snapshots=="true")
      },
      manifest:{path:$manifest_path,sha256:$manifest_sha256},
      local_checks:{path:$local_checks_path,summary:$local_checks[0].summary}
    }' > "$FINAL_BUNDLE_METADATA_FILE"
  chmod 600 "$FINAL_BUNDLE_METADATA_FILE"
  cp -f "$LOCAL_CHECKS_FILE" "$FINAL_BUNDLE_LOCAL_CHECKS_FILE"
  chmod 600 "$FINAL_BUNDLE_LOCAL_CHECKS_FILE"

  jq -n \
    --arg generated_at "$(now_utc)" \
    --arg bundle_name "$FINAL_BUNDLE_NAME" \
    --arg bundle_sha256 "$FINAL_BUNDLE_SHA256" \
    --arg bundle_path "$FINAL_BUNDLE_PATH" \
    --arg local_checks_path "$LOCAL_CHECKS_FILE" \
    '{
      generated_at:$generated_at,
      bundle:{name:$bundle_name,sha256:$bundle_sha256,path:$bundle_path},
      checks:{path:$local_checks_path},
      transport:{http_status:"skipped",websocket_status:"skipped"},
      final_receipt:{status:"pending",receipt_cid:""}
    }' > "$FINAL_BUNDLE_REPORT_FILE"
  chmod 600 "$FINAL_BUNDLE_REPORT_FILE"

  cp -f "$LOCAL_CHECKS_FILE" "$FINAL_LOCAL_CHECKS_FILE"
  cp -f "$FINAL_BUNDLE_METADATA_FILE" "$FINAL_LATEST_METADATA_FILE"
  cp -f "$FINAL_BUNDLE_REPORT_FILE" "$FINAL_LATEST_REPORT_FILE"

  rm -rf "$FINAL_BUNDLE_STAGE_DIR"
  log "ok" "final evidence bundle created: $FINAL_BUNDLE_PATH"
}

transport_final_evidence_bundle() {
  FINAL_HTTP_UPLOAD_STATUS="skipped"
  FINAL_WS_NOTIFY_STATUS="skipped"
  FINAL_HTTP_UPLOAD_NOTE=""
  FINAL_WS_NOTIFY_NOTE=""

  if [[ "$UBL_FINAL_BUNDLE_UPLOAD_ENABLE" != "true" ]]; then
    return 0
  fi
  if [[ -z "$FINAL_BUNDLE_PATH" || ! -f "$FINAL_BUNDLE_PATH" ]]; then
    if [[ "$UBL_FINAL_BUNDLE_REQUIRE_REMOTE" == "true" ]]; then
      log "error" "remote transport requested but final bundle is missing"
      exit 1
    fi
    log "warn" "remote transport requested but final bundle is missing; skipping transport"
    return 0
  fi

  if [[ "$UBL_FINAL_BUNDLE_REQUIRE_REMOTE" == "true" ]]; then
    if [[ -z "$UBL_FINAL_BUNDLE_UPLOAD_HTTP_URL" || -z "$UBL_FINAL_BUNDLE_NOTIFY_WS_URL" ]]; then
      log "error" "strict remote transport requires both HTTP upload and WS notify endpoints"
      exit 1
    fi
  fi
  if [[ -z "$UBL_FINAL_BUNDLE_UPLOAD_HTTP_URL" && -z "$UBL_FINAL_BUNDLE_NOTIFY_WS_URL" ]]; then
    log "error" "final bundle upload enabled but no HTTP or WS endpoint configured"
    exit 1
  fi

  transport_failed="false"

  if [[ -n "$UBL_FINAL_BUNDLE_UPLOAD_HTTP_URL" ]]; then
    http_method="$(printf '%s' "$UBL_FINAL_BUNDLE_UPLOAD_HTTP_METHOD" | tr '[:lower:]' '[:upper:]')"
    if [[ "$http_method" != "PUT" && "$http_method" != "POST" ]]; then
      http_method="PUT"
    fi
    FINAL_HTTP_UPLOAD_RESPONSE_FILE="$UBL_FINAL_BUNDLE_DIR/${FINAL_BUNDLE_NAME}.http.response.txt"

    http_cmd=(curl -fsS -X "$http_method" -H "Content-Type: application/gzip")
    if [[ -n "$UBL_FINAL_BUNDLE_UPLOAD_HTTP_AUTH_HEADER" ]]; then
      http_cmd+=(-H "$UBL_FINAL_BUNDLE_UPLOAD_HTTP_AUTH_HEADER")
    fi
    http_cmd+=(--data-binary "@$FINAL_BUNDLE_PATH" "$UBL_FINAL_BUNDLE_UPLOAD_HTTP_URL")

    if "${http_cmd[@]}" > "$FINAL_HTTP_UPLOAD_RESPONSE_FILE"; then
      FINAL_HTTP_UPLOAD_STATUS="ok"
      FINAL_HTTP_UPLOAD_NOTE="uploaded via HTTP ${http_method}"
      log "ok" "final evidence bundle uploaded via HTTP"
    else
      FINAL_HTTP_UPLOAD_STATUS="fail"
      FINAL_HTTP_UPLOAD_NOTE="http upload failed"
      transport_failed="true"
      log "warn" "final evidence bundle HTTP upload failed"
    fi
  fi

  if [[ -n "$UBL_FINAL_BUNDLE_NOTIFY_WS_URL" ]]; then
    FINAL_WS_NOTIFY_PAYLOAD_FILE="$UBL_FINAL_BUNDLE_DIR/${FINAL_BUNDLE_NAME}.ws.notify.json"
    FINAL_WS_NOTIFY_RESPONSE_FILE="$UBL_FINAL_BUNDLE_DIR/${FINAL_BUNDLE_NAME}.ws.response.txt"
    FINAL_WS_NOTIFY_ERROR_FILE="$UBL_FINAL_BUNDLE_DIR/${FINAL_BUNDLE_NAME}.ws.error.txt"

    jq -n \
      --arg event "ubl.forever_bundle.ready.v1" \
      --arg generated_at "$(now_utc)" \
      --arg release_tag "$UBL_RELEASE_TAG" \
      --arg release_commit "$release_commit" \
      --arg bundle_name "$FINAL_BUNDLE_NAME" \
      --arg bundle_sha256 "$FINAL_BUNDLE_SHA256" \
      --arg bundle_size_bytes "$FINAL_BUNDLE_SIZE_BYTES" \
      --arg api_domain "$UBL_API_DOMAIN" \
      --arg rich_url_domain "$UBL_RICH_URL_DOMAIN" \
      '{
        event:$event,
        generated_at:$generated_at,
        release:{tag:$release_tag,commit:$release_commit},
        domains:{api:$api_domain,rich_url:$rich_url_domain},
        bundle:{name:$bundle_name,sha256:$bundle_sha256,size_bytes:($bundle_size_bytes|tonumber)}
      }' > "$FINAL_WS_NOTIFY_PAYLOAD_FILE"

    if command -v websocat >/dev/null 2>&1; then
      if websocat -n1 -u "$UBL_FINAL_BUNDLE_NOTIFY_WS_URL" < "$FINAL_WS_NOTIFY_PAYLOAD_FILE" > "$FINAL_WS_NOTIFY_RESPONSE_FILE" 2> "$FINAL_WS_NOTIFY_ERROR_FILE"; then
        FINAL_WS_NOTIFY_STATUS="ok"
        FINAL_WS_NOTIFY_NOTE="metadata sent via websocket"
        log "ok" "final evidence websocket notification sent"
      else
        FINAL_WS_NOTIFY_STATUS="fail"
        FINAL_WS_NOTIFY_NOTE="websocket notify failed"
        transport_failed="true"
        log "warn" "final evidence websocket notification failed"
      fi
    else
      FINAL_WS_NOTIFY_STATUS="fail"
      FINAL_WS_NOTIFY_NOTE="websocat missing"
      transport_failed="true"
      log "warn" "websocket notification requested but websocat is not installed"
    fi
  fi

  jq \
    --arg generated_at "$(now_utc)" \
    --arg http_status "$FINAL_HTTP_UPLOAD_STATUS" \
    --arg http_note "$FINAL_HTTP_UPLOAD_NOTE" \
    --arg ws_status "$FINAL_WS_NOTIFY_STATUS" \
    --arg ws_note "$FINAL_WS_NOTIFY_NOTE" \
    '.generated_at=$generated_at
     | .transport.http_status=$http_status
     | .transport.http_note=$http_note
     | .transport.websocket_status=$ws_status
     | .transport.websocket_note=$ws_note' \
    "$FINAL_BUNDLE_REPORT_FILE" > "$FINAL_BUNDLE_REPORT_FILE.tmp"
  mv "$FINAL_BUNDLE_REPORT_FILE.tmp" "$FINAL_BUNDLE_REPORT_FILE"
  cp -f "$FINAL_BUNDLE_REPORT_FILE" "$FINAL_LATEST_REPORT_FILE"

  if [[ "$transport_failed" == "true" && "$UBL_FINAL_BUNDLE_REQUIRE_REMOTE" == "true" ]]; then
    log "error" "remote transport failed and UBL_FINAL_BUNDLE_REQUIRE_REMOTE=true"
    exit 1
  fi
}

emit_final_bootstrap_report_receipt() {
  FINAL_REPORT_RECEIPT_CID=""
  if [[ "$UBL_FINAL_REPORT_RECEIPT_ENABLE" != "true" ]]; then
    log "info" "final report receipt disabled (UBL_FINAL_REPORT_RECEIPT_ENABLE=false)"
    return 0
  fi
  if [[ ! -f "$LOCAL_CHECKS_FILE" || ! -f "$FINAL_BUNDLE_METADATA_FILE" || ! -f "$FINAL_BUNDLE_REPORT_FILE" ]]; then
    if [[ "$UBL_FINAL_REPORT_RECEIPT_REQUIRE" == "true" ]]; then
      log "error" "final report receipt requires local checks + bundle metadata/report files"
      exit 1
    fi
    log "warn" "final report receipt skipped (missing local checks or bundle metadata/report)"
    return 0
  fi

  FINAL_REPORT_CHIP_FILE="$BOOTSTRAP_DIR/final_report.chip.json"
  FINAL_REPORT_RESPONSE_FILE="$BOOTSTRAP_DIR/final_report.response.json"
  FINAL_REPORT_RECEIPT_FILE="$BOOTSTRAP_DIR/final_report.receipt.json"

  final_status="ok"
  if [[ "${LOCAL_CHECKS_FAIL_COUNT:-0}" != "0" ]]; then
    final_status="degraded"
  fi
  if [[ "$FINAL_HTTP_UPLOAD_STATUS" == "fail" || "$FINAL_WS_NOTIFY_STATUS" == "fail" ]]; then
    final_status="degraded"
  fi
  final_id="${UBL_FINAL_REPORT_ID_PREFIX}:${FINAL_BUNDLE_SHA256:0:16}"

  jq -n \
    --arg id "$final_id" \
    --arg world "$UBL_BOOTSTRAP_WORLD" \
    --arg generated_at "$(now_utc)" \
    --arg status "$final_status" \
    --arg api_domain "$UBL_API_DOMAIN" \
    --arg rich_url_domain "$UBL_RICH_URL_DOMAIN" \
    --arg bundle_sha256 "$FINAL_BUNDLE_SHA256" \
    --arg bundle_path "$FINAL_BUNDLE_PATH" \
    --arg metadata_path "$FINAL_BUNDLE_METADATA_FILE" \
    --arg report_path "$FINAL_BUNDLE_REPORT_FILE" \
    --arg local_checks_path "$LOCAL_CHECKS_FILE" \
    --arg http_status "$FINAL_HTTP_UPLOAD_STATUS" \
    --arg ws_status "$FINAL_WS_NOTIFY_STATUS" \
    --slurpfile local_checks "$LOCAL_CHECKS_FILE" \
    --slurpfile metadata "$FINAL_BUNDLE_METADATA_FILE" \
    '{
      "@type":"ubl/document",
      "@id":$id,
      "@ver":"1.0",
      "@world":$world,
      "doc_type":"ubl/bootstrap.final_report.v1",
      "generated_at":$generated_at,
      "status":$status,
      "domains":{api:$api_domain,rich_url:$rich_url_domain},
      "bundle":{
        "sha256":$bundle_sha256,
        "path":$bundle_path,
        "metadata_path":$metadata_path,
        "report_path":$report_path
      },
      "local_checks":{
        "path":$local_checks_path,
        "summary":$local_checks[0].summary
      },
      "transport":{
        "http_status":$http_status,
        "websocket_status":$ws_status
      },
      "metadata":$metadata[0]
    }' > "$FINAL_REPORT_CHIP_FILE"

  if ! curl -fsS -X POST "http://127.0.0.1:4000/v1/chips" \
    -H "content-type: application/json" \
    --data-binary "@$FINAL_REPORT_CHIP_FILE" > "$FINAL_REPORT_RESPONSE_FILE"; then
    if [[ "$UBL_FINAL_REPORT_RECEIPT_REQUIRE" == "true" ]]; then
      log "error" "final bootstrap report chip submit failed"
      exit 1
    fi
    log "warn" "final bootstrap report chip submit failed"
    return 0
  fi

  FINAL_REPORT_RECEIPT_CID="$(jq -r '.receipt_cid // empty' "$FINAL_REPORT_RESPONSE_FILE")"
  if [[ -z "$FINAL_REPORT_RECEIPT_CID" ]]; then
    if [[ "$UBL_FINAL_REPORT_RECEIPT_REQUIRE" == "true" ]]; then
      log "error" "final report response missing receipt_cid"
      exit 1
    fi
    log "warn" "final report response missing receipt_cid"
    return 0
  fi

  if ! fetch_json_with_retry "http://127.0.0.1:4000/v1/receipts/${FINAL_REPORT_RECEIPT_CID}" "$FINAL_REPORT_RECEIPT_FILE" 10 1; then
    if jq -e '.receipt != null' "$FINAL_REPORT_RESPONSE_FILE" >/dev/null 2>&1; then
      jq '.receipt' "$FINAL_REPORT_RESPONSE_FILE" > "$FINAL_REPORT_RECEIPT_FILE"
      log "warn" "final report receipt endpoint unavailable; used embedded receipt"
    else
      if [[ "$UBL_FINAL_REPORT_RECEIPT_REQUIRE" == "true" ]]; then
        log "error" "failed to fetch final report receipt and response has no embedded receipt"
        exit 1
      fi
      log "warn" "failed to fetch final report receipt"
      return 0
    fi
  fi

  jq \
    --arg generated_at "$(now_utc)" \
    --arg status "emitted" \
    --arg receipt_cid "$FINAL_REPORT_RECEIPT_CID" \
    --arg receipt_file "$FINAL_REPORT_RECEIPT_FILE" \
    '.generated_at=$generated_at
     | .final_receipt.status=$status
     | .final_receipt.receipt_cid=$receipt_cid
     | .final_receipt.receipt_file=$receipt_file' \
    "$FINAL_BUNDLE_REPORT_FILE" > "$FINAL_BUNDLE_REPORT_FILE.tmp"
  mv "$FINAL_BUNDLE_REPORT_FILE.tmp" "$FINAL_BUNDLE_REPORT_FILE"
  cp -f "$FINAL_BUNDLE_REPORT_FILE" "$FINAL_LATEST_REPORT_FILE"

  log "ok" "final bootstrap report receipt: $FINAL_REPORT_RECEIPT_CID"
}

append_if_file() {
  local file_path="$1"
  local list_file="$2"
  if [[ -f "$file_path" ]]; then
    printf '%s\n' "$file_path" >> "$list_file"
  fi
}

append_dir_files() {
  local dir_path="$1"
  local list_file="$2"
  if [[ -d "$dir_path" ]]; then
    find "$dir_path" -type f | sort >> "$list_file"
  fi
}

capture_running_services() {
  local out_file="$1"
  : > "$out_file"

  echo "# pm2" >> "$out_file"
  if command -v pm2 >/dev/null 2>&1; then
    pm2 jlist 2>/dev/null | jq -r '.[] | "\(.name // "unknown")\t\(.pm2_env.status // "unknown")\tpid=\(.pid // 0)"' >> "$out_file" || true
  else
    echo "pm2 not available" >> "$out_file"
  fi

  echo "# system services" >> "$out_file"
  if command -v systemctl >/dev/null 2>&1; then
    systemctl list-units --type=service --state=running --no-pager --no-legend >> "$out_file" 2>/dev/null || true
  elif command -v service >/dev/null 2>&1; then
    service --status-all 2>/dev/null >> "$out_file" || true
  else
    echo "system service inventory unavailable" >> "$out_file"
  fi
}

capture_open_ports() {
  local out_file="$1"
  : > "$out_file"
  if command -v ss >/dev/null 2>&1; then
    ss -lntup >> "$out_file" 2>/dev/null || true
  elif command -v lsof >/dev/null 2>&1; then
    lsof -nP -iTCP -sTCP:LISTEN >> "$out_file" 2>/dev/null || true
  elif command -v netstat >/dev/null 2>&1; then
    netstat -lntup >> "$out_file" 2>/dev/null || true
  else
    echo "open port inventory unavailable" >> "$out_file"
  fi
}

capture_packages() {
  local out_file="$1"
  : > "$out_file"

  if command -v dpkg-query >/dev/null 2>&1; then
    dpkg-query -W -f='${Package}\t${Version}\n' | sort >> "$out_file" 2>/dev/null || true
  elif command -v rpm >/dev/null 2>&1; then
    rpm -qa --qf '%{NAME}\t%{VERSION}-%{RELEASE}\n' | sort >> "$out_file" 2>/dev/null || true
  elif command -v brew >/dev/null 2>&1; then
    brew list --versions | sort >> "$out_file" 2>/dev/null || true
  else
    echo "package inventory unavailable" >> "$out_file"
  fi
}

capture_disk_usage() {
  local out_file="$1"
  : > "$out_file"
  {
    echo "captured_at=$(now_utc)"
    echo "df:"
    df -h "$UBL_BASE_DIR"
    echo "du:"
    du -sh "$STATE_DIR" "$LIVE_DIR" "$RELEASE_DIR" 2>/dev/null || true
  } >> "$out_file"
}

write_freeze_manifest() {
  if [[ "$UBL_WRITE_FREEZE_MANIFEST" != "true" ]]; then
    log "warn" "freeze manifest disabled (UBL_WRITE_FREEZE_MANIFEST=false)"
    return 0
  fi

  if [[ -f "$FREEZE_MANIFEST_FILE" && "$UBL_FREEZE_MANIFEST_OVERWRITE" != "true" ]]; then
    log "ok" "freeze manifest already exists: $FREEZE_MANIFEST_FILE"
    return 0
  fi

  local tmpdir files_list tree_jsonl tree_json
  local services_txt ports_txt packages_txt disk_txt
  local services_json ports_json packages_json disk_json
  tmpdir="$(mktemp -d)"
  files_list="$tmpdir/genesis_files.list"
  tree_jsonl="$tmpdir/genesis_tree.jsonl"
  tree_json="$tmpdir/genesis_tree.json"
  services_txt="$tmpdir/running_services.txt"
  ports_txt="$tmpdir/open_ports.txt"
  packages_txt="$tmpdir/packages.txt"
  disk_txt="$tmpdir/disk_usage.txt"
  services_json="$tmpdir/services.json"
  ports_json="$tmpdir/ports.json"
  packages_json="$tmpdir/packages.json"
  disk_json="$tmpdir/disk.json"

  : > "$files_list"
  append_if_file "$MACHINE_BIRTH_FILE" "$files_list"
  append_if_file "$KEY_BIRTH_FILE" "$files_list"
  append_if_file "$RELEASE_PROVENANCE_FILE" "$files_list"
  append_if_file "$GENESIS_PUB_PEM" "$files_list"
  append_if_file "$GENESIS_PUB_SHA_FILE" "$files_list"
  append_if_file "$RUNTIME_ENV" "$files_list"
  append_if_file "$TUNNEL_ENV" "$files_list"
  append_if_file "$ECOSYSTEM_FILE" "$files_list"
  append_if_file "$RELEASE_DIR/bin/ubl_gate.sha256" "$files_list"
  append_dir_files "$RELEASE_DIR/attestation" "$files_list"
  append_dir_files "$BOOTSTRAP_DIR" "$files_list"

  sort -u "$files_list" -o "$files_list"
  : > "$tree_jsonl"
  while IFS= read -r f; do
    [[ -f "$f" ]] || continue
    file_sha="$(sha256_file "$f")"
    file_rel="${f#$UBL_BASE_DIR/}"
    if [[ "$file_rel" == "$f" ]]; then
      file_rel="$f"
    fi
    jq -nc --arg path "$file_rel" --arg sha256 "$file_sha" '{path:$path,sha256:$sha256}' >> "$tree_jsonl"
  done < "$files_list"
  jq -s '.' "$tree_jsonl" > "$tree_json"
  genesis_tree_sha256="$(sha256_file "$tree_json")"

  capture_running_services "$services_txt"
  capture_open_ports "$ports_txt"
  capture_packages "$packages_txt"
  capture_disk_usage "$disk_txt"

  jq -R -s 'split("\n") | map(select(length > 0))' "$services_txt" > "$services_json"
  jq -R -s 'split("\n") | map(select(length > 0))' "$ports_txt" > "$ports_json"
  jq -R -s 'split("\n") | map(select(length > 0))' "$packages_txt" > "$packages_json"
  jq -R -s 'split("\n") | map(select(length > 0))' "$disk_txt" > "$disk_json"

  jq -n \
    --arg generated_at "$(now_utc)" \
    --arg freeze_scope "genesis-layer" \
    --arg host "$(hostname 2>/dev/null || true)" \
    --arg release_tag "$UBL_RELEASE_TAG" \
    --arg release_commit "$release_commit" \
    --arg genesis_layer_tree_sha256 "$genesis_tree_sha256" \
    --arg running_services_sha256 "$(sha256_file "$services_txt")" \
    --arg open_ports_sha256 "$(sha256_file "$ports_txt")" \
    --arg packages_sha256 "$(sha256_file "$packages_txt")" \
    --arg disk_usage_sha256 "$(sha256_file "$disk_txt")" \
    --slurpfile tree "$tree_json" \
    --slurpfile services "$services_json" \
    --slurpfile ports "$ports_json" \
    --slurpfile packages "$packages_json" \
    --slurpfile disk "$disk_json" \
    '{
      generated_at:$generated_at,
      freeze_scope:$freeze_scope,
      host:$host,
      release_tag:$release_tag,
      release_commit:$release_commit,
      genesis_layer:{
        tree_sha256:$genesis_layer_tree_sha256,
        files:$tree[0]
      },
      running_services:{
        sha256:$running_services_sha256,
        values:$services[0]
      },
      open_ports:{
        sha256:$open_ports_sha256,
        values:$ports[0]
      },
      installed_packages:{
        sha256:$packages_sha256,
        values:$packages[0]
      },
      disk_usage:{
        sha256:$disk_usage_sha256,
        values:$disk[0]
      }
    }' > "$FREEZE_MANIFEST_FILE.tmp"
  mv "$FREEZE_MANIFEST_FILE.tmp" "$FREEZE_MANIFEST_FILE"
  chmod 600 "$FREEZE_MANIFEST_FILE"
  rm -rf "$tmpdir"
  log "ok" "freeze manifest written: $FREEZE_MANIFEST_FILE"
}

require_cmd bash
require_cmd gh
require_cmd jq
require_cmd curl
require_cmd tar
require_cmd openssl
require_cmd cargo
require_cmd pm2
require_cmd xxd

if [[ ! -f "$ENV_FILE" ]]; then
  log "error" "env file not found: $ENV_FILE"
  log "info" "copy template: cp ops/forever_bootstrap.env.example ops/forever_bootstrap.env"
  exit 1
fi

# shellcheck disable=SC1090
set -a && source "$ENV_FILE" && set +a

UBL_REPO="${UBL_REPO:-LogLine-Foundation/UBL-CORE}"
UBL_RELEASE_TAG="${UBL_RELEASE_TAG:-v0.1.0-core-baseline}"
UBL_BASE_DIR="${UBL_BASE_DIR:-$HOME/ubl-core-forever}"
LEGACY_UBL_DOMAIN="${UBL_DOMAIN:-}"
UBL_API_DOMAIN="${UBL_API_DOMAIN:-${LEGACY_UBL_DOMAIN:-ubl.agency}}"
UBL_RICH_URL_DOMAIN="${UBL_RICH_URL_DOMAIN:-${LEGACY_UBL_DOMAIN:-$UBL_API_DOMAIN}}"
# Legacy compatibility: UBL_DOMAIN now mirrors rich URL domain.
UBL_DOMAIN="$UBL_RICH_URL_DOMAIN"
UBL_PM2_GATE_APP="${UBL_PM2_GATE_APP:-ubl-gate}"
UBL_PM2_TUNNEL_APP="${UBL_PM2_TUNNEL_APP:-ubl-cloudflared}"
UBL_EMIT_BOOTSTRAP="${UBL_EMIT_BOOTSTRAP:-true}"
UBL_BOOTSTRAP_ID="${UBL_BOOTSTRAP_ID:-logline-world-genesis}"
UBL_BOOTSTRAP_WORLD="${UBL_BOOTSTRAP_WORLD:-a/logline/t/main}"
UBL_EMIT_BOOTSTRAP_ARTIFACT_CHIPS="${UBL_EMIT_BOOTSTRAP_ARTIFACT_CHIPS:-true}"
UBL_REQUIRE_BOOTSTRAP_ARTIFACT_CHIPS="${UBL_REQUIRE_BOOTSTRAP_ARTIFACT_CHIPS:-false}"
UBL_FINAL_CHECKS_REQUIRE_PASS="${UBL_FINAL_CHECKS_REQUIRE_PASS:-false}"
UBL_FINAL_BUNDLE_ENABLE="${UBL_FINAL_BUNDLE_ENABLE:-true}"
UBL_FINAL_BUNDLE_DIR="${UBL_FINAL_BUNDLE_DIR:-}"
UBL_FINAL_BUNDLE_BASENAME="${UBL_FINAL_BUNDLE_BASENAME:-forever-bootstrap-evidence}"
UBL_FINAL_BUNDLE_INCLUDE_SNAPSHOTS="${UBL_FINAL_BUNDLE_INCLUDE_SNAPSHOTS:-false}"
UBL_FINAL_BUNDLE_UPLOAD_ENABLE="${UBL_FINAL_BUNDLE_UPLOAD_ENABLE:-false}"
UBL_FINAL_BUNDLE_UPLOAD_HTTP_URL="${UBL_FINAL_BUNDLE_UPLOAD_HTTP_URL:-}"
UBL_FINAL_BUNDLE_UPLOAD_HTTP_METHOD="${UBL_FINAL_BUNDLE_UPLOAD_HTTP_METHOD:-PUT}"
UBL_FINAL_BUNDLE_UPLOAD_HTTP_AUTH_HEADER="${UBL_FINAL_BUNDLE_UPLOAD_HTTP_AUTH_HEADER:-}"
UBL_FINAL_BUNDLE_NOTIFY_WS_URL="${UBL_FINAL_BUNDLE_NOTIFY_WS_URL:-}"
UBL_FINAL_BUNDLE_REQUIRE_REMOTE="${UBL_FINAL_BUNDLE_REQUIRE_REMOTE:-false}"
UBL_FINAL_REPORT_RECEIPT_ENABLE="${UBL_FINAL_REPORT_RECEIPT_ENABLE:-true}"
UBL_FINAL_REPORT_RECEIPT_REQUIRE="${UBL_FINAL_REPORT_RECEIPT_REQUIRE:-true}"
UBL_FINAL_REPORT_ID_PREFIX="${UBL_FINAL_REPORT_ID_PREFIX:-${UBL_BOOTSTRAP_ID}:final}"
UBL_RECEIPT_QR_ENABLE="${UBL_RECEIPT_QR_ENABLE:-true}"
UBL_RECEIPT_QR_REQUIRE_TOOL="${UBL_RECEIPT_QR_REQUIRE_TOOL:-false}"
UBL_RECEIPT_QR_FORMATS="${UBL_RECEIPT_QR_FORMATS:-png,svg}"
UBL_RECEIPT_QR_ECLEVEL="${UBL_RECEIPT_QR_ECLEVEL:-M}"
UBL_RECEIPT_QR_SCALE="${UBL_RECEIPT_QR_SCALE:-6}"
UBL_CLOUDFLARE_ENABLE="${UBL_CLOUDFLARE_ENABLE:-false}"
UBL_CLOUDFLARE_DNS_ENABLE="${UBL_CLOUDFLARE_DNS_ENABLE:-false}"
UBL_CLOUDFLARE_RECORD_NAME="${UBL_CLOUDFLARE_RECORD_NAME:-$UBL_API_DOMAIN}"
UBL_CLOUDFLARE_RICH_RECORD_ENABLE="${UBL_CLOUDFLARE_RICH_RECORD_ENABLE:-true}"
UBL_CLOUDFLARE_RICH_RECORD_NAME="${UBL_CLOUDFLARE_RICH_RECORD_NAME:-$UBL_RICH_URL_DOMAIN}"
UBL_EXPECTED_GATE_SHA256="${UBL_EXPECTED_GATE_SHA256:-}"
UBL_ATTEST_PUBKEY_SHA256="${UBL_ATTEST_PUBKEY_SHA256:-}"
UBL_ALLOW_UNPINNED_ATTEST_KEY="${UBL_ALLOW_UNPINNED_ATTEST_KEY:-false}"
UBL_TARBALL_SHA256="${UBL_TARBALL_SHA256:-}"
UBL_ALLOW_UNPINNED_TARBALL="${UBL_ALLOW_UNPINNED_TARBALL:-false}"
UBL_ALLOW_KEY_BIRTH_OVERRIDE="${UBL_ALLOW_KEY_BIRTH_OVERRIDE:-false}"
UBL_REQUIRE_EXTERNAL_WITNESS="${UBL_REQUIRE_EXTERNAL_WITNESS:-false}"
UBL_EXTERNAL_WITNESS_BIN="${UBL_EXTERNAL_WITNESS_BIN:-}"
UBL_EXTERNAL_WITNESS_CMD="${UBL_EXTERNAL_WITNESS_CMD:-}"
UBL_ALLOW_SHELL_WITNESS_CMD="${UBL_ALLOW_SHELL_WITNESS_CMD:-false}"
UBL_CLOUDFLARE_SERVICE_INSTALL="${UBL_CLOUDFLARE_SERVICE_INSTALL:-false}"
UBL_CLOUDFLARE_ACCESS_POLICY_CONFIRMED="${UBL_CLOUDFLARE_ACCESS_POLICY_CONFIRMED:-false}"
UBL_CLOUDFLARE_RATE_LIMIT_RULES="${UBL_CLOUDFLARE_RATE_LIMIT_RULES:-}"
CLOUDFLARE_TUNNEL_ORIGIN_URL="${CLOUDFLARE_TUNNEL_ORIGIN_URL:-http://127.0.0.1:4000}"
UBL_BACKUP_ENABLE="${UBL_BACKUP_ENABLE:-true}"
UBL_BACKUP_CRON="${UBL_BACKUP_CRON:-17 3 * * *}"
UBL_BACKUP_DEST="${UBL_BACKUP_DEST:-}"
UBL_BACKUP_ENCRYPTION_PASSPHRASE_FILE="${UBL_BACKUP_ENCRYPTION_PASSPHRASE_FILE:-}"
UBL_BACKUP_ENCRYPTION_MODE="${UBL_BACKUP_ENCRYPTION_MODE:-auto}"
UBL_BACKUP_PBKDF2_ITER="${UBL_BACKUP_PBKDF2_ITER:-600000}"
UBL_BACKUP_INCLUDE_RUNTIME_SECRETS="${UBL_BACKUP_INCLUDE_RUNTIME_SECRETS:-false}"
UBL_HEARTBEAT_ENABLE="${UBL_HEARTBEAT_ENABLE:-true}"
UBL_HEARTBEAT_CRON="${UBL_HEARTBEAT_CRON:-11 2 * * *}"
UBL_HEARTBEAT_WORLD="${UBL_HEARTBEAT_WORLD:-$UBL_BOOTSTRAP_WORLD}"
UBL_HEARTBEAT_ID_PREFIX="${UBL_HEARTBEAT_ID_PREFIX:-logline-world-heartbeat}"
UBL_WRITE_FREEZE_MANIFEST="${UBL_WRITE_FREEZE_MANIFEST:-true}"
UBL_FREEZE_MANIFEST_OVERWRITE="${UBL_FREEZE_MANIFEST_OVERWRITE:-false}"
UBL_SNAPSHOT_INCLUDE_RUNTIME_SECRETS="${UBL_SNAPSHOT_INCLUDE_RUNTIME_SECRETS:-false}"
UBL_SYMBOL_OVERWRITE="${UBL_SYMBOL_OVERWRITE:-false}"

# Cloudflare dual-zone compatibility:
# - Legacy: CLOUDFLARE_API_TOKEN + CLOUDFLARE_ZONE_ID
# - Legacy Global Key alias: CLOUDFLARE_API
# - New: per-domain auth/zone (api + rich)
CLOUDFLARE_API_EMAIL="${CLOUDFLARE_API_EMAIL:-}"
CLOUDFLARE_GLOBAL_API_KEY="${CLOUDFLARE_GLOBAL_API_KEY:-${CLOUDFLARE_API:-}}"

CLOUDFLARE_API_TOKEN_API="${CLOUDFLARE_API_TOKEN_API:-${CLOUDFLARE_API_TOKEN:-}}"
CLOUDFLARE_API_EMAIL_API="${CLOUDFLARE_API_EMAIL_API:-${CLOUDFLARE_API_EMAIL:-}}"
CLOUDFLARE_GLOBAL_API_KEY_API="${CLOUDFLARE_GLOBAL_API_KEY_API:-${CLOUDFLARE_GLOBAL_API_KEY:-}}"
CLOUDFLARE_API_ZONE_ID="${CLOUDFLARE_API_ZONE_ID:-${CLOUDFLARE_ZONE_ID:-}}"

CLOUDFLARE_API_TOKEN_RICH="${CLOUDFLARE_API_TOKEN_RICH:-$CLOUDFLARE_API_TOKEN_API}"
CLOUDFLARE_API_EMAIL_RICH="${CLOUDFLARE_API_EMAIL_RICH:-$CLOUDFLARE_API_EMAIL_API}"
CLOUDFLARE_GLOBAL_API_KEY_RICH="${CLOUDFLARE_GLOBAL_API_KEY_RICH:-$CLOUDFLARE_GLOBAL_API_KEY_API}"
CLOUDFLARE_RICH_ZONE_ID="${CLOUDFLARE_RICH_ZONE_ID:-$CLOUDFLARE_API_ZONE_ID}"

STATE_DIR="$UBL_BASE_DIR/state"
RELEASES_DIR="$UBL_BASE_DIR/releases"
LIVE_DIR="$UBL_BASE_DIR/live"
ART_DIR="$STATE_DIR/downloads/$UBL_RELEASE_TAG"
RELEASE_DIR="$RELEASES_DIR/$UBL_RELEASE_TAG"
RUNTIME_ENV="$LIVE_DIR/config/runtime.env"
TUNNEL_ENV="$LIVE_DIR/config/tunnel.env"
ECOSYSTEM_FILE="$LIVE_DIR/run/ecosystem.config.cjs"
BOOTSTRAP_DIR="$STATE_DIR/bootstrap"
LOG_DIR="$UBL_BASE_DIR/logs"
SNAPSHOT_DIR_ROOT="$STATE_DIR/snapshots"
HEARTBEAT_DIR="$STATE_DIR/heartbeat"
if [[ -z "$UBL_FINAL_BUNDLE_DIR" ]]; then
  UBL_FINAL_BUNDLE_DIR="$STATE_DIR/final"
fi

MACHINE_BIRTH_FILE="$STATE_DIR/machine_birth.json"
KEY_BIRTH_FILE="$STATE_DIR/key_birth.json"
RELEASE_PROVENANCE_FILE="$STATE_DIR/release_provenance.json"
GENESIS_PUB_PEM="$STATE_DIR/genesis_signer.pub.pem"
GENESIS_PUB_SHA_FILE="$STATE_DIR/genesis_signer.pub.pem.sha256"
CLOUDFLARE_RATE_LIMIT_FILE="$STATE_DIR/cloudflare_rate_limit.json"
FREEZE_MANIFEST_FILE="$STATE_DIR/freeze_manifest.json"
SYMBOL_FILE="$STATE_DIR/genesis_symbol.json"
BOOTSTRAP_ARTIFACT_LEDGER_FILE="$BOOTSTRAP_DIR/artifact_chips.json"
BOOTSTRAP_RECEIPT_URL_FILE="$BOOTSTRAP_DIR/receipt_url.txt"
BOOTSTRAP_RECEIPT_URL_RESPONSE_FILE="$BOOTSTRAP_DIR/receipt_url.response.json"
BOOTSTRAP_RECEIPT_URL_PAYLOAD_FILE="$BOOTSTRAP_DIR/receipt_url.payload.json"
BOOTSTRAP_RECEIPT_QR_PNG="$BOOTSTRAP_DIR/receipt_qr.png"
BOOTSTRAP_RECEIPT_QR_SVG="$BOOTSTRAP_DIR/receipt_qr.svg"
FINAL_LOCAL_CHECKS_FILE="$UBL_FINAL_BUNDLE_DIR/local_checks.latest.json"
FINAL_LATEST_METADATA_FILE="$UBL_FINAL_BUNDLE_DIR/final_bundle.latest.metadata.json"
FINAL_LATEST_REPORT_FILE="$UBL_FINAL_BUNDLE_DIR/final_bundle.latest.report.json"

FINAL_BUNDLE_TS=""
FINAL_BUNDLE_NAME=""
FINAL_BUNDLE_STAGE_DIR=""
FINAL_BUNDLE_PATH=""
FINAL_BUNDLE_MANIFEST_FILE=""
FINAL_BUNDLE_METADATA_FILE=""
FINAL_BUNDLE_LOCAL_CHECKS_FILE=""
FINAL_BUNDLE_REPORT_FILE=""
FINAL_BUNDLE_SHA256=""
FINAL_BUNDLE_MANIFEST_SHA256=""
FINAL_BUNDLE_SIZE_BYTES=""
FINAL_HTTP_UPLOAD_STATUS="skipped"
FINAL_WS_NOTIFY_STATUS="skipped"
FINAL_HTTP_UPLOAD_NOTE=""
FINAL_WS_NOTIFY_NOTE=""
FINAL_HTTP_UPLOAD_RESPONSE_FILE=""
FINAL_WS_NOTIFY_PAYLOAD_FILE=""
FINAL_WS_NOTIFY_RESPONSE_FILE=""
FINAL_WS_NOTIFY_ERROR_FILE=""
FINAL_REPORT_CHIP_FILE=""
FINAL_REPORT_RESPONSE_FILE=""
FINAL_REPORT_RECEIPT_FILE=""
FINAL_REPORT_RECEIPT_CID=""
LOCAL_CHECKS_FILE=""
LOCAL_CHECKS_FAIL_COUNT="0"
LOCAL_CHECKS_WARN_COUNT="0"
LOCAL_CHECKS_PASS_COUNT="0"

LOCK_DIR="$UBL_BASE_DIR/.bootstrap.lock"
mkdir -p "$UBL_BASE_DIR"
if ! mkdir "$LOCK_DIR" 2>/dev/null; then
  log "error" "another bootstrap run is active (lock: $LOCK_DIR)"
  exit 1
fi
trap 'rmdir "$LOCK_DIR" >/dev/null 2>&1 || true' EXIT

mkdir -p "$STATE_DIR" "$RELEASES_DIR" "$LIVE_DIR/config" "$LIVE_DIR/run" "$LIVE_DIR/data" "$ART_DIR" "$BOOTSTRAP_DIR" "$LOG_DIR" "$SNAPSHOT_DIR_ROOT" "$HEARTBEAT_DIR" "$UBL_FINAL_BUNDLE_DIR"

log "info" "phase 0: immutable machine birth certificate"
capture_machine_birth

log "info" "phase 1: download public release artifacts"
TARBALL="$ART_DIR/UBL-CORE-${UBL_RELEASE_TAG}.tar.gz"
if [[ ! -f "$TARBALL" ]]; then
  if [[ "$DRY_RUN" == "true" ]]; then
    log "dry-run" "gh api /repos/${UBL_REPO}/tarball/${UBL_RELEASE_TAG} > $TARBALL"
  else
    gh api "/repos/${UBL_REPO}/tarball/${UBL_RELEASE_TAG}" > "$TARBALL"
  fi
  log "ok" "downloaded source tarball"
else
  log "ok" "tarball already present"
fi

run gh release download "$UBL_RELEASE_TAG" \
  --repo "$UBL_REPO" \
  --dir "$ART_DIR" \
  --clobber \
  --pattern manifest.json \
  --pattern attestation.json \
  --pattern attestation.sig \
  --pattern attestation.sig.b64 \
  --pattern docs_attest_signer.pub.pem

if [[ "$DRY_RUN" == "true" ]]; then
  log "info" "dry-run complete: release download/verify/deploy commands were not executed."
  exit 0
fi

require_file "$ART_DIR/manifest.json"
require_file "$ART_DIR/attestation.json"
require_file "$ART_DIR/attestation.sig"
require_file "$ART_DIR/attestation.sig.b64"
require_file "$ART_DIR/docs_attest_signer.pub.pem"

tarball_sha_actual="$(sha256_file "$TARBALL")"
enforce_pinned_hash "tarball sha256" "$tarball_sha_actual" "$UBL_TARBALL_SHA256" "$UBL_ALLOW_UNPINNED_TARBALL"

log "info" "phase 2: verify release attestation"
manifest_sha_expected="$(jq -r '.manifest.sha256 // empty' "$ART_DIR/attestation.json")"
[[ -n "$manifest_sha_expected" ]] || {
  log "error" "attestation missing manifest.sha256"
  exit 1
}
manifest_sha_actual="$(sha256_file "$ART_DIR/manifest.json")"
if [[ "$manifest_sha_actual" != "$manifest_sha_expected" ]]; then
  log "error" "manifest hash mismatch expected=$manifest_sha_expected actual=$manifest_sha_actual"
  exit 1
fi

manifest_tarball_sha="$(jq -r '.source_tarball.sha256 // .source.sha256 // .artifacts.tarball.sha256 // empty' "$ART_DIR/manifest.json")"
if [[ -n "$manifest_tarball_sha" && "$manifest_tarball_sha" != "null" ]]; then
  if [[ "$manifest_tarball_sha" != "$tarball_sha_actual" ]]; then
    log "error" "tarball hash mismatch against manifest expected=$manifest_tarball_sha actual=$tarball_sha_actual"
    exit 1
  fi
  log "ok" "tarball hash matches manifest"
else
  log "warn" "manifest has no tarball hash field; relying on pinned UBL_TARBALL_SHA256"
fi

att_sig_alg="$(jq -r '.signature.alg // empty' "$ART_DIR/attestation.json")"
if [[ -z "$att_sig_alg" ]]; then
  log "error" "attestation missing signature.alg"
  exit 1
fi
if [[ "$att_sig_alg" != "ed25519" ]]; then
  log "error" "unsupported attestation signature algorithm: $att_sig_alg"
  exit 1
fi

att_pub_from_json="$ART_DIR/attestation.pub.pem"
jq -r '.signature.public_key_pem // empty' "$ART_DIR/attestation.json" > "$att_pub_from_json"
if [[ ! -s "$att_pub_from_json" ]]; then
  log "error" "attestation missing signature.public_key_pem"
  exit 1
fi

pub_sha_json="$(sha256_file "$att_pub_from_json")"
pub_sha_asset="$(sha256_file "$ART_DIR/docs_attest_signer.pub.pem")"
if [[ "$pub_sha_json" != "$pub_sha_asset" ]]; then
  log "error" "attestation key mismatch between embedded key and release asset"
  exit 1
fi

enforce_pinned_hash "attestation signer key sha256" "$pub_sha_json" "$UBL_ATTEST_PUBKEY_SHA256" "$UBL_ALLOW_UNPINNED_ATTEST_KEY"

openssl pkeyutl -verify -rawin \
  -pubin -inkey "$ART_DIR/docs_attest_signer.pub.pem" \
  -sigfile "$ART_DIR/attestation.sig" \
  -in "$ART_DIR/manifest.json" >/dev/null
log "ok" "attestation verified"

log "info" "phase 3: extract/build and provenance capture"
release_commit=""
if [[ ! -d "$RELEASE_DIR/src" ]]; then
  tmp_unpack="$RELEASE_DIR/.tmp-unpack"
  run mkdir -p "$tmp_unpack"
  run tar -xzf "$TARBALL" -C "$tmp_unpack"
  src_real="$(find "$tmp_unpack" -mindepth 1 -maxdepth 1 -type d | head -n1)"
  [[ -n "$src_real" ]] || {
    log "error" "failed to locate extracted source dir"
    exit 1
  }

  src_base="$(basename "$src_real")"
  if [[ "$src_base" =~ ([a-f0-9]{40})$ ]]; then
    release_commit="${BASH_REMATCH[1]}"
  fi

  run mkdir -p "$RELEASE_DIR"
  run mv "$src_real" "$RELEASE_DIR/src"
  run rm -rf "$tmp_unpack"

  if [[ -n "$release_commit" ]]; then
    printf '%s\n' "$release_commit" > "$RELEASE_DIR/release_commit.txt"
  fi
else
  if [[ -f "$RELEASE_DIR/release_commit.txt" ]]; then
    release_commit="$(cat "$RELEASE_DIR/release_commit.txt")"
  fi
fi

if [[ -z "$release_commit" && -f "$RELEASE_DIR/release_commit.txt" ]]; then
  release_commit="$(cat "$RELEASE_DIR/release_commit.txt")"
fi
if [[ -z "$release_commit" ]]; then
  release_commit="unknown"
fi

run mkdir -p "$RELEASE_DIR/bin" "$RELEASE_DIR/attestation"
run cp "$ART_DIR/manifest.json" "$RELEASE_DIR/attestation/manifest.json"
run cp "$ART_DIR/attestation.json" "$RELEASE_DIR/attestation/attestation.json"
run cp "$ART_DIR/attestation.sig" "$RELEASE_DIR/attestation/attestation.sig"
run cp "$ART_DIR/attestation.sig.b64" "$RELEASE_DIR/attestation/attestation.sig.b64"
run cp "$ART_DIR/docs_attest_signer.pub.pem" "$RELEASE_DIR/attestation/docs_attest_signer.pub.pem"

if [[ ! -x "$RELEASE_DIR/bin/ubl_gate" ]]; then
  (cd "$RELEASE_DIR/src" && cargo build --release -p ubl_gate)
  cp "$RELEASE_DIR/src/target/release/ubl_gate" "$RELEASE_DIR/bin/ubl_gate"
  chmod +x "$RELEASE_DIR/bin/ubl_gate"
fi

binary_sha=""
binary_match="unknown"
binary_mismatch_reason=""
if [[ -x "$RELEASE_DIR/bin/ubl_gate" ]]; then
  binary_sha="$(sha256_file "$RELEASE_DIR/bin/ubl_gate")"
  printf '%s\n' "$binary_sha" > "$RELEASE_DIR/bin/ubl_gate.sha256"

  if [[ -n "$UBL_EXPECTED_GATE_SHA256" ]]; then
    if [[ "$UBL_EXPECTED_GATE_SHA256" == "$binary_sha" ]]; then
      binary_match="match"
    else
      binary_match="mismatch"
      binary_mismatch_reason="expected hash differs (toolchain/flags/source drift possible)"
      log "warn" "binary hash mismatch expected=$UBL_EXPECTED_GATE_SHA256 actual=$binary_sha"
    fi
  fi
fi

jq -n \
  --arg generated_at "$(now_utc)" \
  --arg release_tag "$UBL_RELEASE_TAG" \
  --arg release_commit "$release_commit" \
  --arg gate_binary_sha256 "$binary_sha" \
  --arg expected_gate_sha256 "$UBL_EXPECTED_GATE_SHA256" \
  --arg reproducibility "$binary_match" \
  --arg reproducibility_note "$binary_mismatch_reason" \
  '{
    generated_at:$generated_at,
    release_tag:$release_tag,
    release_commit:$release_commit,
    gate_binary_sha256:$gate_binary_sha256,
    expected_gate_sha256:$expected_gate_sha256,
    reproducibility:$reproducibility,
    reproducibility_note:$reproducibility_note
  }' > "$RELEASE_PROVENANCE_FILE"

log "info" "phase 4: configure runtime identity/persistence and key birth"
if [[ ! -f "$RUNTIME_ENV" ]]; then
  run mkdir -p "$(dirname "$RUNTIME_ENV")"
  run touch "$RUNTIME_ENV"
  run chmod 600 "$RUNTIME_ENV"
fi

signing_key_hex="$(env_get "SIGNING_KEY_HEX" || true)"
stage_secret="$(env_get "UBL_STAGE_SECRET" || true)"
key_source="existing"

if [[ -z "$signing_key_hex" || -z "$stage_secret" ]]; then
  if [[ -f "$KEY_BIRTH_FILE" && "$UBL_ALLOW_KEY_BIRTH_OVERRIDE" != "true" ]]; then
    log "error" "key_birth.json exists and runtime keys are incomplete; refusing regeneration. Set UBL_ALLOW_KEY_BIRTH_OVERRIDE=true to force."
    exit 1
  fi

  if [[ -z "$signing_key_hex" ]]; then
    signing_key_hex="$(openssl rand -hex 32)"
    upsert_env "SIGNING_KEY_HEX" "$signing_key_hex"
  fi
  if [[ -z "$stage_secret" ]]; then
    stage_secret="hex:$(openssl rand -hex 32)"
    upsert_env "UBL_STAGE_SECRET" "$stage_secret"
  fi

  key_source="generated"
fi

upsert_env "UBL_STORE_BACKEND" "sqlite"
upsert_env "UBL_STORE_DSN" "file:${LIVE_DIR}/data/ubl.db?mode=rwc&_journal_mode=WAL"
upsert_env "RUST_LOG" "info"

run ln -sfn "$RELEASE_DIR" "$LIVE_DIR/current"

if [[ ! -f "$GENESIS_PUB_PEM" ]]; then
  derive_pub_pem_from_seed_hex "$signing_key_hex" "$GENESIS_PUB_PEM"
fi
printf '%s\n' "$(sha256_file "$GENESIS_PUB_PEM")" > "$GENESIS_PUB_SHA_FILE"

if [[ -f "$KEY_BIRTH_FILE" ]]; then
  log "ok" "key birth already exists: $KEY_BIRTH_FILE"
else
  machine_os="$(jq -r '.os_fingerprint // empty' "$MACHINE_BIRTH_FILE")"
  jq -n \
    --arg created_at "$(now_utc)" \
    --arg os_fingerprint "$machine_os" \
    --arg release_tag "$UBL_RELEASE_TAG" \
    --arg release_commit "$release_commit" \
    --arg gate_binary_sha256 "$binary_sha" \
    --arg genesis_pubkey_sha256 "$(sha256_file "$GENESIS_PUB_PEM")" \
    --arg key_source "$key_source" \
    '{
      created_at:$created_at,
      os_fingerprint:$os_fingerprint,
      release_tag:$release_tag,
      release_commit:$release_commit,
      gate_binary_sha256:$gate_binary_sha256,
      genesis_pubkey_sha256:$genesis_pubkey_sha256,
      key_source:$key_source
    }' > "$KEY_BIRTH_FILE"
  chmod 600 "$KEY_BIRTH_FILE"
  log "ok" "key birth captured: $KEY_BIRTH_FILE"
fi

GATE_RUNNER="$LIVE_DIR/run/ubl-gate.sh"
if [[ ! -f "$GATE_RUNNER" ]]; then
  cat > "$GATE_RUNNER" <<RUNNER
#!/usr/bin/env bash
set -euo pipefail
set -a
source "$RUNTIME_ENV"
set +a
exec "$LIVE_DIR/current/bin/ubl_gate"
RUNNER
  chmod +x "$GATE_RUNNER"
fi

if [[ "$UBL_CLOUDFLARE_ENABLE" == "true" ]]; then
  require_cmd cloudflared
  [[ -n "${CLOUDFLARE_TUNNEL_TOKEN:-}" ]] || {
    log "error" "UBL_CLOUDFLARE_ENABLE=true but CLOUDFLARE_TUNNEL_TOKEN is missing"
    exit 1
  }
  if [[ "$UBL_CLOUDFLARE_ACCESS_POLICY_CONFIRMED" != "true" ]]; then
    log "error" "refusing to expose public tunnel without explicit Access confirmation (set UBL_CLOUDFLARE_ACCESS_POLICY_CONFIRMED=true only after Access app/policy is configured)"
    exit 1
  fi

  if [[ ! -f "$TUNNEL_ENV" ]]; then
    run touch "$TUNNEL_ENV"
    run chmod 600 "$TUNNEL_ENV"
  fi
  if ! grep -q '^CLOUDFLARE_TUNNEL_TOKEN=' "$TUNNEL_ENV"; then
    printf 'CLOUDFLARE_TUNNEL_TOKEN=%s\n' "$CLOUDFLARE_TUNNEL_TOKEN" > "$TUNNEL_ENV"
  fi

  TUNNEL_RUNNER="$LIVE_DIR/run/cloudflared.sh"
  TUNNEL_CONFIG_FILE="$LIVE_DIR/config/cloudflared-token.yml"
  if [[ ! -f "$TUNNEL_CONFIG_FILE" ]]; then
    printf '{}\n' > "$TUNNEL_CONFIG_FILE"
    chmod 600 "$TUNNEL_CONFIG_FILE"
  fi
  if [[ -n "$CLOUDFLARE_TUNNEL_ORIGIN_URL" ]]; then
    cat > "$TUNNEL_RUNNER" <<RUNNER
#!/usr/bin/env bash
set -euo pipefail
source "$TUNNEL_ENV"
exec cloudflared tunnel --config "$TUNNEL_CONFIG_FILE" run --token "\$CLOUDFLARE_TUNNEL_TOKEN" --url "$CLOUDFLARE_TUNNEL_ORIGIN_URL"
RUNNER
  else
    cat > "$TUNNEL_RUNNER" <<RUNNER
#!/usr/bin/env bash
set -euo pipefail
source "$TUNNEL_ENV"
exec cloudflared tunnel --config "$TUNNEL_CONFIG_FILE" run --token "\$CLOUDFLARE_TUNNEL_TOKEN"
RUNNER
  fi
  chmod +x "$TUNNEL_RUNNER"

  # Trade-off note:
  # - PM2-managed tunnel is simple but depends on PM2 startup ordering.
  # - cloudflared system service is usually more resilient on reboot.
  if [[ "$UBL_CLOUDFLARE_SERVICE_INSTALL" == "true" ]]; then
    if ! cloudflared service install >/tmp/ubl_cloudflared_service_install.log 2>&1; then
      log "warn" "cloudflared service install failed; check /tmp/ubl_cloudflared_service_install.log"
    else
      log "ok" "cloudflared system service installed"
    fi
  else
    log "info" "cloudflared system service not installed (UBL_CLOUDFLARE_SERVICE_INSTALL=false)"
  fi
fi

# Single-write ecosystem generation (avoid diverging double-write blocks).
{
  echo "module.exports = {"
  echo "  apps: ["
  cat <<APP_GATE
    {
      name: "${UBL_PM2_GATE_APP}",
      script: "${GATE_RUNNER}",
      cwd: "${LIVE_DIR}",
      autorestart: true,
      max_restarts: 10,
      min_uptime: "5s",
      out_file: "${LOG_DIR}/${UBL_PM2_GATE_APP}.out.log",
      error_file: "${LOG_DIR}/${UBL_PM2_GATE_APP}.err.log"
    }
APP_GATE

  if [[ "$UBL_CLOUDFLARE_ENABLE" == "true" ]]; then
    cat <<APP_TUNNEL
    ,{
      name: "${UBL_PM2_TUNNEL_APP}",
      script: "${LIVE_DIR}/run/cloudflared.sh",
      cwd: "${LIVE_DIR}",
      autorestart: true,
      max_restarts: 20,
      min_uptime: "5s",
      out_file: "${LOG_DIR}/${UBL_PM2_TUNNEL_APP}.out.log",
      error_file: "${LOG_DIR}/${UBL_PM2_TUNNEL_APP}.err.log"
    }
APP_TUNNEL
  fi

  echo "  ]"
  echo "}"
} > "$ECOSYSTEM_FILE"

log "info" "phase 5: start/restart services (pm2)"
if pm2 describe "$UBL_PM2_GATE_APP" >/dev/null 2>&1; then
  run pm2 restart "$UBL_PM2_GATE_APP" --update-env
else
  run pm2 start "$ECOSYSTEM_FILE" --only "$UBL_PM2_GATE_APP"
fi

if [[ "$UBL_CLOUDFLARE_ENABLE" == "true" ]]; then
  if pm2 describe "$UBL_PM2_TUNNEL_APP" >/dev/null 2>&1; then
    run pm2 restart "$UBL_PM2_TUNNEL_APP" --update-env
  else
    run pm2 start "$ECOSYSTEM_FILE" --only "$UBL_PM2_TUNNEL_APP"
  fi
fi

run pm2 save

log "info" "phase 5b: pm2 startup registration"
if ! pm2 startup >/tmp/ubl_pm2_startup.log 2>&1; then
  startup_cmd="$(grep -Eo 'sudo .*pm2 startup[^\"]*' /tmp/ubl_pm2_startup.log | head -n1 || true)"
  if [[ -n "$startup_cmd" ]]; then
    log "warn" "pm2 startup needs manual root step: $startup_cmd"
  else
    log "warn" "pm2 startup command could not auto-register; check /tmp/ubl_pm2_startup.log"
  fi
else
  log "ok" "pm2 startup registered"
fi

log "info" "phase 5c: pm2 logrotate baseline"
if ! pm2 conf pm2-logrotate >/dev/null 2>&1; then
  pm2 install pm2-logrotate >/dev/null 2>&1 || true
fi
pm2 set pm2-logrotate:retain 90 >/dev/null 2>&1 || true
pm2 set pm2-logrotate:compress true >/dev/null 2>&1 || true
pm2 set pm2-logrotate:rotateInterval '0 0 * * *' >/dev/null 2>&1 || true

log "info" "phase 6: local health checks"
ok="false"
for _ in $(seq 1 60); do
  if curl --connect-timeout 2 --max-time 5 -fsS "http://127.0.0.1:4000/healthz" >/dev/null 2>&1; then
    ok="true"
    break
  fi
  sleep 1
done
if [[ "$ok" != "true" ]]; then
  log "error" "gate healthz failed on localhost:4000"
  exit 1
fi
log "ok" "local healthz passed"

log "info" "phase 6b: schema marker capture"
schema_hash="unavailable"
if command -v sqlite3 >/dev/null 2>&1 && [[ -f "$LIVE_DIR/data/ubl.db" ]]; then
  schema_sql="$(sqlite3 "$LIVE_DIR/data/ubl.db" "SELECT sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY name;" || true)"
  if [[ -n "$schema_sql" ]]; then
    schema_hash="$(sha256_text "$schema_sql")"
    log "ok" "schema marker captured from sqlite_master"
  else
    log "warn" "sqlite present but schema DDL extraction returned empty; using schema_version=unavailable"
  fi
else
  log "warn" "sqlite schema marker unavailable (sqlite3 missing or $LIVE_DIR/data/ubl.db absent)"
fi

jq \
  --arg schema_version "$schema_hash" \
  --arg schema_captured_at "$(now_utc)" \
  '.schema_version=$schema_version | .schema_captured_at=$schema_captured_at' \
  "$RELEASE_PROVENANCE_FILE" > "$RELEASE_PROVENANCE_FILE.tmp"
mv "$RELEASE_PROVENANCE_FILE.tmp" "$RELEASE_PROVENANCE_FILE"

if [[ "$UBL_CLOUDFLARE_DNS_ENABLE" == "true" ]]; then
  if [[ -z "${CLOUDFLARE_API_TOKEN_API:-}" && ( -z "${CLOUDFLARE_API_EMAIL_API:-}" || -z "${CLOUDFLARE_GLOBAL_API_KEY_API:-}" ) ]]; then
    log "error" "Cloudflare API auth missing: set CLOUDFLARE_API_TOKEN_API (or CLOUDFLARE_API_TOKEN) OR CLOUDFLARE_API_EMAIL_API+CLOUDFLARE_GLOBAL_API_KEY_API (legacy: CLOUDFLARE_API_EMAIL+CLOUDFLARE_API)"
    exit 1
  fi
  [[ -n "${CLOUDFLARE_API_ZONE_ID:-}" ]] || { log "error" "CLOUDFLARE_API_ZONE_ID (or CLOUDFLARE_ZONE_ID) missing"; exit 1; }
  [[ -n "${CLOUDFLARE_TUNNEL_ID:-}" ]] || { log "error" "CLOUDFLARE_TUNNEL_ID missing"; exit 1; }
  if [[ "$UBL_CLOUDFLARE_RICH_RECORD_ENABLE" == "true" ]]; then
    if [[ -z "${CLOUDFLARE_API_TOKEN_RICH:-}" && ( -z "${CLOUDFLARE_API_EMAIL_RICH:-}" || -z "${CLOUDFLARE_GLOBAL_API_KEY_RICH:-}" ) ]]; then
      log "error" "Cloudflare RICH auth missing: set CLOUDFLARE_API_TOKEN_RICH OR CLOUDFLARE_API_EMAIL_RICH+CLOUDFLARE_GLOBAL_API_KEY_RICH"
      exit 1
    fi
    [[ -n "${CLOUDFLARE_RICH_ZONE_ID:-}" ]] || { log "error" "CLOUDFLARE_RICH_ZONE_ID missing while rich DNS record is enabled"; exit 1; }
  fi

  log "info" "phase 7: ensure Cloudflare DNS CNAME"
  target="${CLOUDFLARE_TUNNEL_ID}.cfargotunnel.com"
  cf_api="https://api.cloudflare.com/client/v4"

  upsert_cloudflare_cname_record "$cf_api" "$CLOUDFLARE_API_TOKEN_API" "$CLOUDFLARE_API_EMAIL_API" "$CLOUDFLARE_GLOBAL_API_KEY_API" "$CLOUDFLARE_API_ZONE_ID" "$UBL_CLOUDFLARE_RECORD_NAME" "$target"
  if [[ "$UBL_CLOUDFLARE_RICH_RECORD_ENABLE" == "true" && -n "$UBL_CLOUDFLARE_RICH_RECORD_NAME" ]]; then
    upsert_cloudflare_cname_record "$cf_api" "$CLOUDFLARE_API_TOKEN_RICH" "$CLOUDFLARE_API_EMAIL_RICH" "$CLOUDFLARE_GLOBAL_API_KEY_RICH" "$CLOUDFLARE_RICH_ZONE_ID" "$UBL_CLOUDFLARE_RICH_RECORD_NAME" "$target"
  fi
fi

if [[ "$UBL_CLOUDFLARE_ENABLE" == "true" ]]; then
  if [[ -n "$UBL_CLOUDFLARE_RATE_LIMIT_RULES" ]]; then
    jq -n \
      --arg generated_at "$(now_utc)" \
      --arg domain "$UBL_API_DOMAIN" \
      --arg rules "$UBL_CLOUDFLARE_RATE_LIMIT_RULES" \
      '{generated_at:$generated_at,domain:$domain,rules:$rules}' > "$CLOUDFLARE_RATE_LIMIT_FILE"
    log "ok" "cloudflare rate-limit registry saved: $CLOUDFLARE_RATE_LIMIT_FILE"
  else
    log "warn" "no Cloudflare rate-limit rule registry provided (set UBL_CLOUDFLARE_RATE_LIMIT_RULES)"
  fi
fi

if [[ "$UBL_CLOUDFLARE_ENABLE" == "true" ]]; then
  ok="false"
  for _ in $(seq 1 90); do
    if curl --connect-timeout 3 --max-time 8 -fsS "https://${UBL_API_DOMAIN}/healthz" >/dev/null 2>&1; then
      ok="true"
      break
    fi
    sleep 2
  done
  if [[ "$ok" == "true" ]]; then
    log "ok" "public tunnel healthz passed"
  else
    log "warn" "public tunnel healthz not confirmed yet"
  fi
fi

RESPONSE_FILE="$BOOTSTRAP_DIR/response.json"
receipt_cid=""
if [[ "$UBL_EMIT_BOOTSTRAP" == "true" ]]; then
  log "info" "phase 8: emit canonical bootstrap receipt"

  if [[ -s "$BOOTSTRAP_DIR/receipt.json" ]]; then
    log "ok" "bootstrap receipt already present; skipping re-emission"
    if [[ -f "$BOOTSTRAP_DIR/index.json" ]]; then
      receipt_cid="$(jq -r '.receipt_cid // empty' "$BOOTSTRAP_DIR/index.json")"
    fi
  else
    if [[ -f "$BOOTSTRAP_DIR/receipt.json" && ! -s "$BOOTSTRAP_DIR/receipt.json" ]]; then
      log "warn" "found empty bootstrap receipt artifact; re-emitting bootstrap receipt"
    fi
    PAYLOAD_FILE="$BOOTSTRAP_DIR/bootstrap_chip.json"
    if [[ ! -f "$PAYLOAD_FILE" ]]; then
      ts="$(now_utc)"
      cat > "$PAYLOAD_FILE" <<EOF_JSON
{
  "@type": "ubl/document",
  "@id": "${UBL_BOOTSTRAP_ID}",
  "@ver": "1.0",
  "@world": "${UBL_BOOTSTRAP_WORLD}",
  "title": "LogLine Forever Bootstrap",
  "domain": "${UBL_RICH_URL_DOMAIN}",
  "api_domain": "${UBL_API_DOMAIN}",
  "rich_url_domain": "${UBL_RICH_URL_DOMAIN}",
  "release_tag": "${UBL_RELEASE_TAG}",
  "host": "LAB-512",
  "created_at": "${ts}",
  "notes": "First canonical bootstrap receipt for permanent trajectory."
}
EOF_JSON
    fi

    curl -fsS -X POST "http://127.0.0.1:4000/v1/chips" \
      -H "content-type: application/json" \
      --data-binary "@$PAYLOAD_FILE" > "$RESPONSE_FILE"

    receipt_cid="$(jq -r '.receipt_cid // empty' "$RESPONSE_FILE")"
    if [[ -z "$receipt_cid" ]]; then
      log "error" "bootstrap response missing receipt_cid"
      cat "$RESPONSE_FILE"
      exit 1
    fi

    if ! fetch_json_with_retry "http://127.0.0.1:4000/v1/receipts/${receipt_cid}" "$BOOTSTRAP_DIR/receipt.json" 10 1; then
      # Fallback: receipt may already be embedded in /v1/chips response.
      if jq -e '.receipt != null' "$RESPONSE_FILE" >/dev/null 2>&1; then
        jq '.receipt' "$RESPONSE_FILE" > "$BOOTSTRAP_DIR/receipt.json"
        log "warn" "receipt endpoint unavailable; used embedded receipt from /v1/chips response"
      else
        log "error" "failed to fetch receipt and response has no embedded receipt"
        exit 1
      fi
    fi

    if ! fetch_json_with_retry "http://127.0.0.1:4000/v1/receipts/${receipt_cid}/trace" "$BOOTSTRAP_DIR/trace.json" 10 1; then
      jq -n --arg warning "trace endpoint unavailable after retries" --arg receipt_cid "$receipt_cid" '{warning:$warning,receipt_cid:$receipt_cid}' > "$BOOTSTRAP_DIR/trace.json"
      log "warn" "trace endpoint unavailable; wrote warning trace artifact"
    fi
    if ! fetch_json_with_retry "http://127.0.0.1:4000/v1/receipts/${receipt_cid}/narrate" "$BOOTSTRAP_DIR/narrate.json" 10 1; then
      jq -n --arg warning "narrate endpoint unavailable after retries" --arg receipt_cid "$receipt_cid" '{warning:$warning,receipt_cid:$receipt_cid}' > "$BOOTSTRAP_DIR/narrate.json"
      log "warn" "narrate endpoint unavailable; wrote warning narrate artifact"
    fi
    log "ok" "bootstrap receipt: $receipt_cid"
  fi
fi

if [[ -z "$receipt_cid" && -f "$BOOTSTRAP_DIR/index.json" ]]; then
  receipt_cid="$(jq -r '.receipt_cid // empty' "$BOOTSTRAP_DIR/index.json")"
fi
if [[ -z "$receipt_cid" && -f "$BOOTSTRAP_DIR/response.json" ]]; then
  receipt_cid="$(jq -r '.receipt_cid // empty' "$BOOTSTRAP_DIR/response.json")"
fi

bootstrap_receipt_url=""
if [[ -f "$BOOTSTRAP_DIR/receipt.json" ]]; then
  log "info" "phase 8b: canonical public receipt URL + QR"
  receipt_url_model=""
  rm -f "$BOOTSTRAP_RECEIPT_URL_RESPONSE_FILE"

  if [[ -f "$RESPONSE_FILE" ]] && jq -e '.receipt_url != null and .receipt_url != ""' "$RESPONSE_FILE" >/dev/null 2>&1; then
    jq '{receipt_url:.receipt_url,receipt_public:.receipt_public}' "$RESPONSE_FILE" > "$BOOTSTRAP_RECEIPT_URL_RESPONSE_FILE"
  elif [[ -n "$receipt_cid" ]] && fetch_json_with_retry "http://127.0.0.1:4000/v1/receipts/${receipt_cid}/url" "$BOOTSTRAP_RECEIPT_URL_RESPONSE_FILE" 10 1; then
    :
  elif [[ -f "$BOOTSTRAP_DIR/index.json" ]] && jq -e '.receipt_url != null and .receipt_url != ""' "$BOOTSTRAP_DIR/index.json" >/dev/null 2>&1; then
    jq '{receipt_url:.receipt_url,receipt_public:{model:(.receipt_url_model // "legacy")}}' "$BOOTSTRAP_DIR/index.json" > "$BOOTSTRAP_RECEIPT_URL_RESPONSE_FILE"
    log "warn" "using cached receipt URL from existing index.json (core URL endpoint unavailable)"
  else
    log "error" "core did not emit canonical receipt URL (missing /v1/receipts/:cid/url and response.receipt_url)"
    exit 1
  fi

  bootstrap_receipt_url="$(jq -r '.receipt_url // empty' "$BOOTSTRAP_RECEIPT_URL_RESPONSE_FILE")"
  if [[ -z "$bootstrap_receipt_url" ]]; then
    log "error" "receipt URL response missing receipt_url"
    cat "$BOOTSTRAP_RECEIPT_URL_RESPONSE_FILE"
    exit 1
  fi
  receipt_url_model="$(jq -r '.receipt_public.model // "unknown"' "$BOOTSTRAP_RECEIPT_URL_RESPONSE_FILE")"
  if jq -e '.receipt_public.payload != null' "$BOOTSTRAP_RECEIPT_URL_RESPONSE_FILE" >/dev/null 2>&1; then
    jq '.receipt_public.payload' "$BOOTSTRAP_RECEIPT_URL_RESPONSE_FILE" > "$BOOTSTRAP_RECEIPT_URL_PAYLOAD_FILE"
    chmod 600 "$BOOTSTRAP_RECEIPT_URL_PAYLOAD_FILE"
  else
    rm -f "$BOOTSTRAP_RECEIPT_URL_PAYLOAD_FILE"
  fi

  printf '%s\n' "$bootstrap_receipt_url" > "$BOOTSTRAP_RECEIPT_URL_FILE"
  chmod 600 "$BOOTSTRAP_RECEIPT_URL_FILE"
  generate_receipt_qr_artifacts "$bootstrap_receipt_url" "$BOOTSTRAP_RECEIPT_QR_PNG" "$BOOTSTRAP_RECEIPT_QR_SVG"

  jq -n \
    --arg generated_at "$(now_utc)" \
    --arg receipt_cid "$receipt_cid" \
    --arg receipt_url "$bootstrap_receipt_url" \
    --arg receipt_url_model "$receipt_url_model" \
    --arg local_receipt "http://127.0.0.1:4000/v1/receipts/${receipt_cid}" \
    --arg local_console "http://127.0.0.1:4000/console/receipt/${receipt_cid}" \
    '{
      generated_at:$generated_at,
      receipt_cid:$receipt_cid,
      receipt_url:$receipt_url,
      receipt_url_model:$receipt_url_model,
      local_receipt:$local_receipt,
      local_console:$local_console
    }' > "$BOOTSTRAP_DIR/index.json"
  chmod 600 "$BOOTSTRAP_DIR/index.json"
  log "ok" "public receipt URL: $bootstrap_receipt_url (model=$receipt_url_model)"
fi

if [[ -f "$BOOTSTRAP_DIR/receipt.json" ]]; then
  log "info" "phase 9: external witness materialization"
  receipt_sha="$(sha256_file "$BOOTSTRAP_DIR/receipt.json")"
  pub_sha="$(sha256_file "$GENESIS_PUB_PEM")"
  witness_file="$BOOTSTRAP_DIR/witness.json"

  jq -n \
    --arg generated_at "$(now_utc)" \
    --arg receipt0_sha256 "$receipt_sha" \
    --arg genesis_pubkey_sha256 "$pub_sha" \
    --arg release_commit "$release_commit" \
    --arg receipt_cid "$receipt_cid" \
    '{
      generated_at:$generated_at,
      receipt0_sha256:$receipt0_sha256,
      genesis_pubkey_sha256:$genesis_pubkey_sha256,
      release_commit:$release_commit,
      receipt_cid:$receipt_cid
    }' > "$witness_file"

  witness_ok="false"
  if [[ -n "$UBL_EXTERNAL_WITNESS_BIN" ]]; then
    if [[ ! -x "$UBL_EXTERNAL_WITNESS_BIN" ]]; then
      log "error" "external witness bin is not executable: $UBL_EXTERNAL_WITNESS_BIN"
      exit 1
    fi
    if WITNESS_FILE="$witness_file" "$UBL_EXTERNAL_WITNESS_BIN" "$witness_file"; then
      witness_ok="true"
      log "ok" "external witness command executed via binary"
    else
      log "warn" "external witness binary execution failed"
    fi
  elif [[ -n "$UBL_EXTERNAL_WITNESS_CMD" ]]; then
    if [[ "$UBL_ALLOW_SHELL_WITNESS_CMD" != "true" ]]; then
      log "error" "UBL_EXTERNAL_WITNESS_CMD is set but shell execution is disabled (set UBL_ALLOW_SHELL_WITNESS_CMD=true to allow legacy mode)"
      exit 1
    fi
    log "warn" "executing legacy shell witness command; prefer UBL_EXTERNAL_WITNESS_BIN"
    if WITNESS_FILE="$witness_file" sh -c "$UBL_EXTERNAL_WITNESS_CMD"; then
      witness_ok="true"
      log "ok" "external witness command executed"
    else
      log "warn" "external witness command failed"
    fi
  fi

  if [[ "$witness_ok" != "true" ]]; then
    if [[ "$UBL_REQUIRE_EXTERNAL_WITNESS" == "true" ]]; then
      log "error" "external witness is required but was not successfully published"
      exit 1
    fi
    log "warn" "publish witness externally now: $witness_file"
  fi
fi

if [[ -f "$BOOTSTRAP_DIR/receipt.json" && -f "$GENESIS_PUB_PEM" ]]; then
  log "info" "phase 9b: genesis symbol materialization"
  if [[ -f "$SYMBOL_FILE" && "$UBL_SYMBOL_OVERWRITE" != "true" ]]; then
    log "ok" "genesis symbol already exists: $SYMBOL_FILE"
  else
    symbol_receipt_sha="$(sha256_file "$BOOTSTRAP_DIR/receipt.json")"
    symbol_pub_sha="$(sha256_file "$GENESIS_PUB_PEM")"
    symbol_receipt_cid="$receipt_cid"
    if [[ -z "$symbol_receipt_cid" && -f "$BOOTSTRAP_DIR/index.json" ]]; then
      symbol_receipt_cid="$(jq -r '.receipt_cid // empty' "$BOOTSTRAP_DIR/index.json")"
    fi
    jq -n \
      --arg generated_at "$(now_utc)" \
      --arg release_tag "$UBL_RELEASE_TAG" \
      --arg release_commit "$release_commit" \
      --arg genesis_pubkey_sha256 "$symbol_pub_sha" \
      --arg receipt0_sha256 "$symbol_receipt_sha" \
      --arg receipt_cid "$symbol_receipt_cid" \
      --arg trust_anchor_pub_pem "$GENESIS_PUB_PEM" \
      --arg receipt_path "$BOOTSTRAP_DIR/receipt.json" \
      --arg witness_path "$BOOTSTRAP_DIR/witness.json" \
      --arg key_birth_path "$KEY_BIRTH_FILE" \
      --arg provenance_path "$RELEASE_PROVENANCE_FILE" \
      '{
        generated_at:$generated_at,
        release_tag:$release_tag,
        release_commit:$release_commit,
        symbol:{
          genesis_pubkey_sha256:$genesis_pubkey_sha256,
          receipt0_sha256:$receipt0_sha256,
          receipt_cid:$receipt_cid
        },
        references:{
          trust_anchor_pub_pem:$trust_anchor_pub_pem,
          receipt:$receipt_path,
          witness:$witness_path,
          key_birth:$key_birth_path,
          release_provenance:$provenance_path
        }
      }' > "$SYMBOL_FILE.tmp"
    mv "$SYMBOL_FILE.tmp" "$SYMBOL_FILE"
    chmod 600 "$SYMBOL_FILE"
    log "ok" "genesis symbol written: $SYMBOL_FILE"
  fi
fi

if [[ -f "$BOOTSTRAP_DIR/receipt.json" ]]; then
  log "info" "phase 10: immediate snapshot after bootstrap"
  snap_ts="$(date -u +%Y%m%dT%H%M%SZ)"
  snap_dir="$SNAPSHOT_DIR_ROOT/$snap_ts"
  if [[ ! -d "$snap_dir" ]]; then
    mkdir -p "$snap_dir"
    write_redacted_runtime_env "$RUNTIME_ENV" "$snap_dir/runtime.public.env"
    chmod 600 "$snap_dir/runtime.public.env"
    if [[ "$UBL_SNAPSHOT_INCLUDE_RUNTIME_SECRETS" == "true" ]]; then
      cp -f "$RUNTIME_ENV" "$snap_dir/runtime.env"
      chmod 600 "$snap_dir/runtime.env"
      log "warn" "snapshot includes runtime secrets (UBL_SNAPSHOT_INCLUDE_RUNTIME_SECRETS=true)"
    else
      log "info" "snapshot excludes runtime secrets by default (runtime.public.env only)"
    fi
    if [[ -f "$LIVE_DIR/data/ubl.db" ]]; then
      cp -f "$LIVE_DIR/data/ubl.db" "$snap_dir/ubl.db"
    fi
    if [[ -f "$LIVE_DIR/data/ubl.db-wal" ]]; then
      cp -f "$LIVE_DIR/data/ubl.db-wal" "$snap_dir/ubl.db-wal"
    fi
    if [[ -f "$LIVE_DIR/data/ubl.db-shm" ]]; then
      cp -f "$LIVE_DIR/data/ubl.db-shm" "$snap_dir/ubl.db-shm"
    fi
    cp -a "$BOOTSTRAP_DIR" "$snap_dir/bootstrap"
    cp -a "$RELEASE_DIR/attestation" "$snap_dir/attestation"
    log "warn" "snapshot created at $snap_dir; copy to encrypted off-site storage NOW"
  else
    log "ok" "snapshot already exists: $snap_dir"
  fi
fi

log "info" "phase 11: backup schedule from day one"
if [[ "$UBL_BACKUP_ENABLE" == "true" ]]; then
  [[ -n "$UBL_BACKUP_DEST" ]] || { log "error" "UBL_BACKUP_DEST must be set when UBL_BACKUP_ENABLE=true"; exit 1; }
  [[ -n "$UBL_BACKUP_ENCRYPTION_PASSPHRASE_FILE" ]] || { log "error" "UBL_BACKUP_ENCRYPTION_PASSPHRASE_FILE must be set"; exit 1; }
  [[ -f "$UBL_BACKUP_ENCRYPTION_PASSPHRASE_FILE" ]] || { log "error" "passphrase file not found: $UBL_BACKUP_ENCRYPTION_PASSPHRASE_FILE"; exit 1; }

  mkdir -p "$UBL_BACKUP_DEST"
  BACKUP_RUNNER="$LIVE_DIR/run/backup_sqlite.sh"
  cat > "$BACKUP_RUNNER" <<BACKUP
#!/usr/bin/env bash
set -euo pipefail
TS="\$(date -u +%Y%m%dT%H%M%SZ)"
STATE_DIR="$STATE_DIR"
LIVE_DIR="$LIVE_DIR"
DEST_DIR="$UBL_BACKUP_DEST"
PASS_FILE="$UBL_BACKUP_ENCRYPTION_PASSPHRASE_FILE"
ENC_MODE="$UBL_BACKUP_ENCRYPTION_MODE"
PBKDF2_ITER="$UBL_BACKUP_PBKDF2_ITER"
INCLUDE_RUNTIME_SECRETS="$UBL_BACKUP_INCLUDE_RUNTIME_SECRETS"

mkdir -p "\$DEST_DIR"
stage_dir="\$(mktemp -d)"
trap 'rm -rf "\$stage_dir"' EXIT

awk -F= '
  BEGIN {
    redacted["SIGNING_KEY_HEX"]=1
    redacted["UBL_STAGE_SECRET"]=1
    redacted["CLOUDFLARE_TUNNEL_TOKEN"]=1
  }
  /^[A-Za-z_][A-Za-z0-9_]*=/ {
    key=\$1
    if (key in redacted) {
      print key "=[REDACTED]"
    } else {
      print \$0
    }
    next
  }
  { print \$0 }
' "$RUNTIME_ENV" > "\$stage_dir/runtime.public.env"
if [[ "\$INCLUDE_RUNTIME_SECRETS" == "true" ]]; then
  cp -f "$RUNTIME_ENV" "\$stage_dir/runtime.env"
fi
storage_mode=""
if [[ -f "$LIVE_DIR/data/ubl.db" ]]; then
  cp -f "$LIVE_DIR/data/ubl.db" "\$stage_dir/ubl.db"
  [[ -f "$LIVE_DIR/data/ubl.db-wal" ]] && cp -f "$LIVE_DIR/data/ubl.db-wal" "\$stage_dir/ubl.db-wal"
  [[ -f "$LIVE_DIR/data/ubl.db-shm" ]] && cp -f "$LIVE_DIR/data/ubl.db-shm" "\$stage_dir/ubl.db-shm"
  storage_mode="sqlite"
elif [[ -d "$LIVE_DIR/data" ]]; then
  cp -a "$LIVE_DIR/data" "\$stage_dir/data"
  storage_mode="fs-tree"
else
  echo "[error] no runtime data found at $LIVE_DIR/data" >&2
  exit 1
fi
[[ -d "$BOOTSTRAP_DIR" ]] && cp -a "$BOOTSTRAP_DIR" "\$stage_dir/bootstrap"
[[ -d "$RELEASE_DIR/attestation" ]] && cp -a "$RELEASE_DIR/attestation" "\$stage_dir/attestation"

raw_archive="\$DEST_DIR/ubl-backup-\$TS.tar.gz"
enc_archive="\$raw_archive.enc"
meta_archive="\$enc_archive.meta.json"

tar -czf "\$raw_archive" -C "\$stage_dir" .
mode_used=""
if [[ "\$ENC_MODE" == "auto" || "\$ENC_MODE" == "openssl-gcm" ]]; then
  if openssl enc -ciphers | tr ' ' '\n' | grep -qx -- '-aes-256-gcm'; then
    if openssl enc -aes-256-gcm -pbkdf2 -iter "\$PBKDF2_ITER" -md sha256 -salt -in "\$raw_archive" -out "\$enc_archive" -pass file:"\$PASS_FILE" 2>/dev/null; then
      mode_used="openssl-aes-256-gcm"
    elif [[ "\$ENC_MODE" == "openssl-gcm" ]]; then
      echo "[error] openssl-gcm mode requested but encryption failed" >&2
      exit 1
    fi
  elif [[ "\$ENC_MODE" == "openssl-gcm" ]]; then
    echo "[error] openssl-gcm mode requested but unsupported by local openssl" >&2
    exit 1
  fi
fi

if [[ -z "\$mode_used" ]]; then
  openssl enc -aes-256-cbc -pbkdf2 -iter "\$PBKDF2_ITER" -md sha256 -salt -in "\$raw_archive" -out "\$enc_archive" -pass file:"\$PASS_FILE"
  # Integrity sidecar for CBC fallback.
  hmac_key="\$(cat "\$PASS_FILE")"
  if ! openssl dgst -sha256 -hmac "\$hmac_key" "\$enc_archive" | awk '{print \$2}' > "\$enc_archive.hmac"; then
    echo "[error] failed to compute HMAC sidecar for CBC fallback" >&2
    exit 1
  fi
  mode_used="openssl-aes-256-cbc+hmac-sha256"
fi

  jq -n \
    --arg generated_at "\$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg mode "\$mode_used" \
  --arg archive "\$enc_archive" \
  --arg storage_mode "\$storage_mode" \
  --arg include_runtime_secrets "\$INCLUDE_RUNTIME_SECRETS" \
  --arg sha256 "\$( if command -v sha256sum >/dev/null 2>&1; then sha256sum "\$enc_archive" | awk '{print \$1}'; else shasum -a 256 "\$enc_archive" | awk '{print \$1}'; fi )" \
    '{generated_at:\$generated_at,mode:\$mode,archive:\$archive,storage_mode:\$storage_mode,include_runtime_secrets:(\$include_runtime_secrets=="true"),sha256:\$sha256}' > "\$meta_archive"
rm -f "\$raw_archive"

echo "\$enc_archive"
BACKUP
  chmod +x "$BACKUP_RUNNER"

  install_or_replace_cron_line "UBL-FOREVER-BACKUP" "$UBL_BACKUP_CRON" "bash $BACKUP_RUNNER >> $LOG_DIR/backup.log 2>&1"

  if [[ "$DRY_RUN" != "true" ]]; then
    "$BACKUP_RUNNER" >/tmp/ubl_backup_first_run.log
    first_backup="$(tail -n1 /tmp/ubl_backup_first_run.log 2>/dev/null || true)"
    if [[ -z "$first_backup" || ! -f "$first_backup" ]]; then
      log "error" "backup first run did not produce encrypted artifact"
      exit 1
    fi
    log "ok" "backup installed and first encrypted artifact created: $first_backup"
  fi
else
  log "warn" "backup is disabled (UBL_BACKUP_ENABLE=false)"
fi

log "info" "phase 12: heartbeat receipts scheduler"
if [[ "$UBL_HEARTBEAT_ENABLE" == "true" ]]; then
  HEARTBEAT_RUNNER="$LIVE_DIR/run/heartbeat_receipt.sh"
  cat > "$HEARTBEAT_RUNNER" <<HEART
#!/usr/bin/env bash
set -euo pipefail
TS="\$(date -u +%Y-%m-%dT%H:%M:%SZ)"
RID="${UBL_HEARTBEAT_ID_PREFIX}-\$(date -u +%Y%m%dT%H%M%SZ)"
HB_DIR="$HEARTBEAT_DIR"
mkdir -p "\$HB_DIR"

payload="\$HB_DIR/payload.json"
cat > "\$payload" <<JSON
{
  "@type": "ubl/heartbeat",
  "@id": "\$RID",
  "@ver": "1.0",
  "@world": "${UBL_HEARTBEAT_WORLD}",
  "ts": "\$TS",
  "host": "LAB-512"
}
JSON

resp="\$HB_DIR/latest_response.json"
curl -fsS -X POST "http://127.0.0.1:4000/v1/chips" -H "content-type: application/json" --data-binary "@\$payload" > "\$resp"
cid="\$(jq -r '.receipt_cid // empty' "\$resp")"

jq -n --arg ts "\$TS" --arg id "\$RID" --arg cid "\$cid" '{ts:\$ts,id:\$id,receipt_cid:\$cid}' > "\$HB_DIR/latest.json"
printf '%s\n' "\$(cat "\$HB_DIR/latest.json")" >> "\$HB_DIR/history.jsonl"
HEART
  chmod +x "$HEARTBEAT_RUNNER"
  install_or_replace_cron_line "UBL-FOREVER-HEARTBEAT" "$UBL_HEARTBEAT_CRON" "bash $HEARTBEAT_RUNNER >> $LOG_DIR/heartbeat.log 2>&1"
else
  log "warn" "heartbeat receipts disabled (UBL_HEARTBEAT_ENABLE=false)"
fi

log "info" "phase 13: freeze marker"
write_freeze_manifest

log "info" "phase 14: bootstrap artifact chips (dogfooding)"
emit_bootstrap_artifact_chips

log "info" "phase 15: local bootstrap checks"
LOCAL_CHECKS_FILE="$UBL_FINAL_BUNDLE_DIR/local_checks.$(date -u +%Y%m%dT%H%M%SZ).json"
run_local_bootstrap_checks "$LOCAL_CHECKS_FILE"

log "info" "phase 16: final evidence bundle"
build_final_evidence_bundle

log "info" "phase 17: optional remote transport"
transport_final_evidence_bundle

log "info" "phase 18: final bootstrap report receipt"
emit_final_bootstrap_report_receipt

log "ok" "forever bootstrap complete"
log "ok" "release dir: $RELEASE_DIR"
log "ok" "live env: $RUNTIME_ENV"
log "ok" "bootstrap artifacts: $BOOTSTRAP_DIR"
log "ok" "machine birth: $MACHINE_BIRTH_FILE"
log "ok" "key birth: $KEY_BIRTH_FILE"
log "ok" "public trust anchor: $GENESIS_PUB_PEM ($GENESIS_PUB_SHA_FILE)"
log "ok" "release provenance: $RELEASE_PROVENANCE_FILE"
log "ok" "freeze marker: $FREEZE_MANIFEST_FILE"
log "ok" "artifact chip ledger: $BOOTSTRAP_ARTIFACT_LEDGER_FILE"
if [[ -f "$BOOTSTRAP_RECEIPT_URL_FILE" ]]; then
  log "ok" "canonical receipt URL: $(cat "$BOOTSTRAP_RECEIPT_URL_FILE")"
fi
if [[ -f "$BOOTSTRAP_RECEIPT_QR_PNG" || -f "$BOOTSTRAP_RECEIPT_QR_SVG" ]]; then
  log "ok" "receipt QR artifacts: png=$BOOTSTRAP_RECEIPT_QR_PNG svg=$BOOTSTRAP_RECEIPT_QR_SVG"
fi
log "ok" "domains: api=$UBL_API_DOMAIN rich_url=$UBL_RICH_URL_DOMAIN"
log "ok" "local checks: $LOCAL_CHECKS_FILE (pass=$LOCAL_CHECKS_PASS_COUNT warn=$LOCAL_CHECKS_WARN_COUNT fail=$LOCAL_CHECKS_FAIL_COUNT)"
if [[ -n "$FINAL_BUNDLE_PATH" ]]; then
  log "ok" "final bundle: $FINAL_BUNDLE_PATH (sha256=$FINAL_BUNDLE_SHA256)"
  log "ok" "final bundle metadata: $FINAL_BUNDLE_METADATA_FILE"
  log "ok" "final bundle report: $FINAL_BUNDLE_REPORT_FILE"
fi
if [[ -n "$FINAL_REPORT_RECEIPT_CID" ]]; then
  log "ok" "final report receipt cid: $FINAL_REPORT_RECEIPT_CID"
fi
