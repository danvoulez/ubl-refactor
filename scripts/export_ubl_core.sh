#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  export_ubl_core.sh --src <source_repo> --dst <target_dir> [options]

Options:
  --profile <strict|max>   Extraction profile (default: strict)
  --dry-run                Show planned copy without writing files
  --validate               Run core validation commands in destination
  --no-normalize-name      Skip post-copy naming normalization (default: normalize)
  -h, --help               Show this help

Profiles:
  strict  Excludes product/specialization track (docs/visao, vcx script)
  max     Copies almost everything except VCS/build artifacts
EOF
}

SRC=""
DST=""
PROFILE="strict"
DRY_RUN=0
VALIDATE=0
NORMALIZE_NAME=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --src)
      SRC="${2:-}"
      shift 2
      ;;
    --dst)
      DST="${2:-}"
      shift 2
      ;;
    --profile)
      PROFILE="${2:-}"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --validate)
      VALIDATE=1
      shift
      ;;
    --no-normalize-name)
      NORMALIZE_NAME=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$SRC" || -z "$DST" ]]; then
  echo "Error: --src and --dst are required." >&2
  usage
  exit 1
fi

if [[ "$PROFILE" != "strict" && "$PROFILE" != "max" ]]; then
  echo "Error: --profile must be 'strict' or 'max'." >&2
  exit 1
fi

if [[ ! -d "$SRC/.git" ]]; then
  echo "Error: --src must be a git repo root (missing .git): $SRC" >&2
  exit 1
fi

SRC="$(cd "$SRC" && pwd)"
DST="$(mkdir -p "$DST" && cd "$DST" && pwd)"

if [[ "$DST" == "$SRC" ]]; then
  echo "Error: --dst must be different from --src." >&2
  exit 1
fi

echo "Source:      $SRC"
echo "Destination: $DST"
echo "Profile:     $PROFILE"
echo "Dry run:     $DRY_RUN"
echo "Validate:    $VALIDATE"
echo "Normalize:   $NORMALIZE_NAME"

RSYNC_OPTS=(-a --delete --prune-empty-dirs)
if [[ $DRY_RUN -eq 1 ]]; then
  RSYNC_OPTS+=(-n -v)
fi

EXCLUDES=(
  --exclude ".git/"
  --exclude ".DS_Store"
  --exclude "target/"
  --exclude "**/target/"
)

if [[ "$PROFILE" == "strict" ]]; then
  EXCLUDES+=(
    --exclude "docs/visao/"
    --exclude "scripts/vcx_conformance.sh"
  )
fi

rsync "${RSYNC_OPTS[@]}" "${EXCLUDES[@]}" "$SRC/" "$DST/"

if [[ $DRY_RUN -eq 0 ]]; then
  if command -v git >/dev/null 2>&1; then
    (cd "$SRC" && git rev-parse HEAD) > "$DST/UPSTREAM_COMMIT.txt"
  fi
  cat > "$DST/CORE_PROFILE.txt" <<EOF
profile=$PROFILE
exported_at_utc=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
source=$SRC
EOF
fi

if [[ $DRY_RUN -eq 0 && $NORMALIZE_NAME -eq 1 ]]; then
  if [[ -f "$DST/Cargo.toml" ]]; then
    perl -0777 -i -pe 's/name = "ubl_master"/name = "ubl_core"/g' "$DST/Cargo.toml"
  fi
  if [[ -f "$DST/README.md" ]]; then
    perl -0777 -i -pe 's/\bUBL MASTER\b/UBL CORE/g' "$DST/README.md"
  fi
  if [[ -f "$DST/services/ubl_gate/src/main.rs" ]]; then
    perl -0777 -i -pe 's/\bubl-master\b/ubl-core/g' "$DST/services/ubl_gate/src/main.rs"
  fi
fi

if [[ $DRY_RUN -eq 0 ]]; then
  for required in \
    "$DST/crates/ubl_cli" \
    "$DST/crates/ubl_chipstore" \
    "$DST/crates/ubl_eventstore" \
    "$DST/crates/ubl_config" \
    "$DST/crates/ubl_kms" \
    "$DST/services/ubl_gate" \
    "$DST/logline"; do
    if [[ ! -e "$required" ]]; then
      echo "Error: required core component missing after export: $required" >&2
      exit 1
    fi
  done
fi

if [[ $VALIDATE -eq 1 && $DRY_RUN -eq 0 ]]; then
  echo "Running validation in destination..."
  (
    cd "$DST"
    cargo check --workspace
    cargo test --workspace --no-run
    cargo run -p ubl_cli -- --help >/dev/null
  )
  (
    cd "$DST"
    cargo run -p ubl_gate >/tmp/ubl_core_export_gate.log 2>&1 &
    gate_pid=$!
    cleanup_gate() { kill "$gate_pid" >/dev/null 2>&1 || true; }
    trap cleanup_gate EXIT
    ok=0
    for _ in $(seq 1 60); do
      if curl -fsS "http://127.0.0.1:4000/healthz" >/tmp/ubl_core_export_healthz.json 2>/dev/null; then
        ok=1
        break
      fi
      sleep 1
    done
    if [[ $ok -ne 1 ]]; then
      echo "Error: gate did not answer /healthz during validation" >&2
      exit 1
    fi
    mcp_status="$(curl -sS -o /tmp/ubl_core_export_mcp.json -w '%{http_code}' \
      -X POST "http://127.0.0.1:4000/mcp/rpc" \
      -H 'content-type: application/json' \
      --data '{"jsonrpc":"2.0","id":"probe","method":"ping"}')"
    if [[ "$mcp_status" == "404" ]]; then
      echo "Error: /mcp/rpc endpoint missing (404)" >&2
      exit 1
    fi
  )
  echo "Validation passed."
fi

echo "Done."
