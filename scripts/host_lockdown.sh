#!/usr/bin/env bash
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="${FOREVER_ENV_FILE:-${SCRIPT_DIR}/../ops/forever_bootstrap.env}"
DRY_RUN="false"

usage() {
  cat <<USAGE
Constitutional host lockdown for UBL forever host.

Usage:
  scripts/host_lockdown.sh [--env <file>] [--dry-run]

Notes:
  - Must run as root/admin.
  - Creates service user model + optional break-glass controls.
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

write_file() {
  local path="$1"
  local content="$2"
  if [[ "$DRY_RUN" == "true" ]]; then
    log "dry-run" "write file $path"
    return 0
  fi
  printf '%s\n' "$content" > "$path"
}

require_cmd() {
  local c="$1"
  command -v "$c" >/dev/null 2>&1 || {
    log "error" "missing command: $c"
    exit 1
  }
}

if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
  log "error" "host_lockdown must run as root/admin"
  exit 1
fi

if [[ -f "$ENV_FILE" ]]; then
  # shellcheck disable=SC1090
  set -a && source "$ENV_FILE" && set +a
else
  log "warn" "env file not found ($ENV_FILE), using defaults"
fi

UBL_SERVICE_USER="${UBL_SERVICE_USER:-ubl}"
UBL_SERVICE_GROUP="${UBL_SERVICE_GROUP:-$UBL_SERVICE_USER}"
UBL_SERVICE_HOME="${UBL_SERVICE_HOME:-/var/lib/ubl}"
UBL_BASE_DIR="${UBL_BASE_DIR:-$UBL_SERVICE_HOME}"
UBL_STATE_DIR="${UBL_STATE_DIR:-$UBL_BASE_DIR/state}"
UBL_SNAPSHOTS_DIR="${UBL_SNAPSHOTS_DIR:-$UBL_BASE_DIR/snapshots}"
UBL_LOG_DIR="${UBL_LOG_DIR:-/var/log/ubl}"
UBL_OPERATOR_USER="${UBL_OPERATOR_USER:-}"
UBL_CREATE_OPERATOR="${UBL_CREATE_OPERATOR:-false}"
UBL_ENABLE_SUDOERS_POLICY="${UBL_ENABLE_SUDOERS_POLICY:-false}"
UBL_INSTALL_MAINTENANCE_CMDS="${UBL_INSTALL_MAINTENANCE_CMDS:-true}"
UBL_PM2_STARTUP_REGISTER="${UBL_PM2_STARTUP_REGISTER:-true}"
UBL_MARK_GENESIS_IMMUTABLE="${UBL_MARK_GENESIS_IMMUTABLE:-false}"
UBL_BREAKGLASS_ADMIN="${UBL_BREAKGLASS_ADMIN:-}"
UBL_FAIL_ON_OPERATOR_BROAD_SUDO="${UBL_FAIL_ON_OPERATOR_BROAD_SUDO:-true}"

require_cmd id
require_cmd getent
require_cmd useradd
require_cmd usermod
require_cmd groupadd
require_cmd mkdir
require_cmd chown
require_cmd chmod

log "info" "phase 1: service identity"
if ! getent group "$UBL_SERVICE_GROUP" >/dev/null 2>&1; then
  run groupadd --system "$UBL_SERVICE_GROUP"
fi

if id -u "$UBL_SERVICE_USER" >/dev/null 2>&1; then
  run usermod --home "$UBL_SERVICE_HOME" --shell /usr/sbin/nologin "$UBL_SERVICE_USER"
else
  run useradd --system --home "$UBL_SERVICE_HOME" --shell /usr/sbin/nologin --gid "$UBL_SERVICE_GROUP" "$UBL_SERVICE_USER"
fi

log "info" "phase 2: directory ownership and permissions"
run mkdir -p "$UBL_BASE_DIR" "$UBL_STATE_DIR" "$UBL_SNAPSHOTS_DIR" "$UBL_LOG_DIR"
run chown -R "$UBL_SERVICE_USER:$UBL_SERVICE_GROUP" "$UBL_BASE_DIR" "$UBL_LOG_DIR"
run chmod 750 "$UBL_BASE_DIR"
run chmod 700 "$UBL_STATE_DIR" "$UBL_SNAPSHOTS_DIR"
run chmod 750 "$UBL_LOG_DIR"

if [[ "$UBL_CREATE_OPERATOR" == "true" && -n "$UBL_OPERATOR_USER" ]]; then
  log "info" "phase 3: operator user"
  if ! id -u "$UBL_OPERATOR_USER" >/dev/null 2>&1; then
    run useradd --create-home --shell /bin/bash "$UBL_OPERATOR_USER"
  fi
fi

if [[ -n "$UBL_OPERATOR_USER" ]] && id -u "$UBL_OPERATOR_USER" >/dev/null 2>&1; then
  log "info" "phase 3b: operator broad sudo audit"
  broad_sudo="false"
  for grp in sudo wheel admin; do
    if id -nG "$UBL_OPERATOR_USER" | tr ' ' '\n' | grep -qx "$grp"; then
      log "warn" "operator user belongs to privileged group '$grp': $UBL_OPERATOR_USER"
      broad_sudo="true"
    fi
  done
  if command -v sudo >/dev/null 2>&1; then
    if sudo -l -U "$UBL_OPERATOR_USER" 2>/dev/null | grep -Eq 'NOPASSWD: ALL|\(ALL(:ALL)?\)[[:space:]]+ALL'; then
      log "warn" "operator user has broad sudo grants from sudoers rules: $UBL_OPERATOR_USER"
      broad_sudo="true"
    fi
  else
    log "warn" "sudo command not found; skipping sudoers broad grant audit"
  fi
  if [[ "$broad_sudo" == "true" && "$UBL_FAIL_ON_OPERATOR_BROAD_SUDO" == "true" ]]; then
    log "error" "operator broad sudo detected; remove broad grants or set UBL_FAIL_ON_OPERATOR_BROAD_SUDO=false"
    exit 1
  fi
fi

if [[ "$UBL_INSTALL_MAINTENANCE_CMDS" == "true" ]]; then
  log "info" "phase 4: maintenance command interface"

  maint_status="#!/usr/bin/env bash
set -euo pipefail
SERVICE_USER=\"$UBL_SERVICE_USER\"
STATE_DIR=\"$UBL_STATE_DIR\"

if command -v runuser >/dev/null 2>&1; then
  runuser -u \"$UBL_SERVICE_USER\" -- pm2 status || true
else
  su -s /bin/bash -c \"pm2 status\" \"$UBL_SERVICE_USER\" || true
fi

if command -v curl >/dev/null 2>&1; then
  curl -fsS http://127.0.0.1:4000/healthz || true
fi

echo \"state_dir=$UBL_STATE_DIR\"
stat \"$UBL_STATE_DIR\" || true
"
  write_file "/usr/local/bin/ubl-maint-status" "$maint_status"
  run chmod 750 /usr/local/bin/ubl-maint-status
  run chown root:root /usr/local/bin/ubl-maint-status

  maint_snapshot="#!/usr/bin/env bash
set -euo pipefail
STATE_DIR=\"$UBL_STATE_DIR\"
OUT_DIR=\"\${1:-}\"

if [[ -z \"\$OUT_DIR\" ]]; then
  echo \"usage: ubl-export-snapshot <out-dir>\" >&2
  exit 1
fi
mkdir -p \"\$OUT_DIR\"
TS=\$(date -u +%Y%m%dT%H%M%SZ)
OUT=\"\$OUT_DIR/ubl-state-snapshot-\$TS.tar.gz\"

tar -czf \"\$OUT\" -C \"\$STATE_DIR\" .
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum \"\$OUT\" > \"\$OUT.sha256\"
else
  shasum -a 256 \"\$OUT\" > \"\$OUT.sha256\"
fi
echo \"\$OUT\"
"
  write_file "/usr/local/bin/ubl-export-snapshot" "$maint_snapshot"
  run chmod 750 /usr/local/bin/ubl-export-snapshot
  run chown root:root /usr/local/bin/ubl-export-snapshot
fi

if [[ "$UBL_ENABLE_SUDOERS_POLICY" == "true" && -n "$UBL_OPERATOR_USER" ]]; then
  log "info" "phase 5: sudoers policy for maintenance-only operator"
  require_cmd visudo
  sudoers_file="/etc/sudoers.d/90-ubl-operator-maint"
  sudoers_content="# Managed by scripts/host_lockdown.sh
Cmnd_Alias UBL_MAINT = /usr/local/bin/ubl-maint-status, /usr/local/bin/ubl-export-snapshot
$UBL_OPERATOR_USER ALL=(root) NOPASSWD: UBL_MAINT
"
  write_file "$sudoers_file" "$sudoers_content"
  run chmod 440 "$sudoers_file"
  run visudo -cf "$sudoers_file"
fi

if [[ "$UBL_BREAKGLASS_ADMIN" != "" ]]; then
  log "info" "phase 6: break-glass admin presence"
  if ! id -u "$UBL_BREAKGLASS_ADMIN" >/dev/null 2>&1; then
    log "warn" "break-glass admin user does not exist: $UBL_BREAKGLASS_ADMIN"
  else
    log "ok" "break-glass admin user exists: $UBL_BREAKGLASS_ADMIN"
  fi
fi

if [[ "$UBL_PM2_STARTUP_REGISTER" == "true" ]]; then
  log "info" "phase 7: PM2 startup registration for service user"
  if command -v pm2 >/dev/null 2>&1; then
    if ! env PATH="$PATH" pm2 startup systemd -u "$UBL_SERVICE_USER" --hp "$UBL_SERVICE_HOME" >/tmp/ubl_pm2_startup_service.log 2>&1; then
      log "warn" "pm2 startup for service user requires manual follow-up; see /tmp/ubl_pm2_startup_service.log"
    else
      log "ok" "pm2 startup registration command executed"
    fi
  else
    log "warn" "pm2 not installed; skipping PM2 startup registration"
  fi
fi

if [[ "$UBL_MARK_GENESIS_IMMUTABLE" == "true" ]]; then
  log "info" "phase 8: immutable genesis markers"
  if command -v chattr >/dev/null 2>&1; then
    for f in machine_birth.json key_birth.json genesis_signer.pub.pem genesis_signer.pub.pem.sha256 release_provenance.json; do
      if [[ -f "$UBL_STATE_DIR/$f" ]]; then
        run chattr +i "$UBL_STATE_DIR/$f"
      fi
    done
  else
    log "warn" "chattr not available; cannot set immutable flags"
  fi
fi

log "ok" "host lockdown complete"
log "ok" "service_user=$UBL_SERVICE_USER"
log "ok" "state_dir=$UBL_STATE_DIR"
log "ok" "operator_user=${UBL_OPERATOR_USER:-<none>}"
