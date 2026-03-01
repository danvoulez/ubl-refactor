#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<USAGE
UBL Ops command router.

Usage:
  scripts/ubl_ops.sh <command> [args]

Commands:
  bootstrap-core   Run core/bootstrap flow (forever_bootstrap.sh)
  host-lockdown    Run host lockdown hardening (host_lockdown.sh)
  ops-maintenance  Run maintenance cleanup/report flow (workzone_cleanup.sh)
  source-flow      Run source distribution flow (source_flow.sh)

Examples:
  scripts/ubl_ops.sh bootstrap-core --env ./ops/forever_bootstrap.env
  sudo scripts/ubl_ops.sh host-lockdown --env ./ops/forever_bootstrap.env
  scripts/ubl_ops.sh ops-maintenance --env ./ops/forever_bootstrap.env --dry-run
  scripts/ubl_ops.sh source-flow publish --env ./ops/source_flow.env
USAGE
}

cmd="${1:-}"
if [[ -z "$cmd" ]]; then
  usage
  exit 1
fi
shift || true

case "$cmd" in
  bootstrap-core)
    exec bash "$SCRIPT_DIR/forever_bootstrap.sh" "$@"
    ;;
  host-lockdown)
    exec bash "$SCRIPT_DIR/host_lockdown.sh" "$@"
    ;;
  ops-maintenance)
    exec bash "$SCRIPT_DIR/workzone_cleanup.sh" "$@"
    ;;
  source-flow)
    exec bash "$SCRIPT_DIR/source_flow.sh" "$@"
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    echo "[error] unknown command: $cmd" >&2
    usage
    exit 1
    ;;
esac
