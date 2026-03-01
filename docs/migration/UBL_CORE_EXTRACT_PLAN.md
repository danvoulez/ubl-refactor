# UBL-CORE Extract Plan

**Status**: proposed  
**Owner**: Core Runtime  
**Date**: 2026-02-21

## Goal

Define a reproducible extraction of `UBL-CORE` from `ubl-master` into a new folder outside this codebase, preserving the current core runtime behavior and excluding product-specialization tracks.

## Recommended Core Boundary (Profile: `strict`)

### Include

- Workspace core runtime code:
  - `crates/`
  - `services/ubl_gate/`
  - `logline/` (connector/parser peripheral used by ecosystem tooling)
- Core contracts and validation assets:
  - `schemas/`
  - `kats/`
- Core operations and specs:
  - `ops/`
  - `specs/`
  - `scripts/` (except product-specific scripts below)
- Documentation needed for core governance/ops/reference:
  - `docs/` (except `docs/visao/`)
- Root files:
  - `Cargo.toml`
  - `Cargo.lock`
  - `Makefile`
  - `README.md`
  - `START-HERE.md`
  - `ARCHITECTURE.md`
  - `TASKLIST.md`
  - `SECURITY.md`
  - `GOVERNANCE.md`
  - `CERTIFIED_RUNTIME.md`
  - `ADDENDUM_CERTIFIED_RUNTIME.md`
  - `ROLLOUT_P0_TO_P1.md`
  - `P0_GENESIS_POLICY.json`
  - `P1_POLICY_UPDATE.json`
  - `UNC-1.md`
  - `.gitignore`
  - `.github/`
  - `src/`

### Exclude

- Product/vertical track:
  - `docs/visao/**`
  - `scripts/vcx_conformance.sh`
- Build/output and VCS internals:
  - `.git/`
  - `target/`
  - `**/target/`
  - `.DS_Store`

## Why this boundary

- The root workspace (`Cargo.toml`) already defines the runnable core surface:
  - `crates/*` + `services/ubl_gate`
- `docs/visao/*` is outside the root workspace and represents specialization/standardization tracks.
- `cargo check --workspace` succeeds on this core surface (validated on 2026-02-21).

## Mandatory Runtime Surface (must remain in UBL-CORE)

- CLI:
  - `crates/ubl_cli` (binary `ublx`)
- MCP:
  - `POST /mcp/rpc` in `services/ubl_gate`
- Peripherals/connectors:
  - `crates/ubl_chipstore` (storage backends/query/indexing)
  - `crates/ubl_eventstore`
  - `crates/ubl_config`
  - `crates/ubl_kms`
  - `logline/`

## Naming Normalization

After extraction, normalize provisional naming so `ubl-master` does not remain as product identity in UBL-CORE:

- Root package name:
  - `Cargo.toml`: `name = "ubl_master"` -> `name = "ubl_core"`
- Public branding strings:
  - `README.md`: `UBL MASTER` -> `UBL CORE`
  - Gate health payload/system label:
    - `services/ubl_gate/src/main.rs`: `ubl-master` -> `ubl-core`

## Copy Procedure

Use:

```bash
bash scripts/export_ubl_core.sh \
  --src /Users/ubl-ops/ubl-master2 \
  --dst /Users/ubl-ops/UBL-CORE \
  --profile strict \
  --validate
```

## Acceptance Gates (must pass in destination)

1. `cargo check --workspace`
2. `cargo test --workspace --no-run`
3. `cargo run -p ubl_cli -- --help` works
4. `cargo run -p ubl_gate` starts without missing file/template errors
5. `GET /healthz` returns success
6. `POST /mcp/rpc` returns non-404 (endpoint wired)

## Operational Notes

- Keep `UPSTREAM_COMMIT.txt` in destination for traceability.
- If you need a near-mirror extraction (including `docs/visao`), use profile `max`.
- After extraction, optionally trim `docs/INDEX.md` entries that point to `docs/visao/*` for cleaner core-only docs navigation.
