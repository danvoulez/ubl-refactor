# START-HERE (LLM-First Canonical Index)

**Status**: active  
**Owner**: Core Runtime  
**Last reviewed**: 2026-02-24

Single entry point for LLMs and humans.
If you read only one document to navigate UBL-CORE docs, read this one.

## 0) Navigation Contract

- This file is the top-level index.
- `START-HERE.md` is the non-negotiable runtime contract.
- `docs/INDEX.md` is the extended catalog.
- `docs/canon/CANON-REFERENCE.md` is the canon registry.

## 1) Mission-Critical Order (Read First)

1. `START-HERE.md`
2. `ARCHITECTURE.md`
3. `ARCHITECTURE_v2.md`
4. `TASKLIST.md`
5. `docs/canon/CANON-REFERENCE.md`

## 2) Canonical Source-of-Truth Map

- System constitution and invariants: `ARCHITECTURE.md`
- Consolidated architecture v2 and fractal model: `ARCHITECTURE_v2.md`
- Execution status and backlog: `TASKLIST.md`
- Canon registry (topic -> spec -> code -> evidence): `docs/canon/CANON-REFERENCE.md`
- Numeric canon (UNC-1): `docs/canon/UNC-1.md`
- Runtime/API shape: `openapi.json` + `crates/ubl_runtime/src/manifest.rs`
- Error taxonomy: `crates/ubl_runtime/src/error_response.rs`
- Security/trust model: `SECURITY.md`
- Documentation policy: `docs/STANDARDS.md`

## 3) Cross-Links by Entry Document

- Entered through `README.md`:
  - Go to `docs/START-HERE-LLM-FIRST.md`, then `START-HERE.md`.

- Entered through `ARCHITECTURE_v2.md`:
  - Validate runtime invariants in `ARCHITECTURE.md` and canon links in `docs/canon/CANON-REFERENCE.md`.

- Entered through `docs/canon/UNC-1.md`:
  - Cross-check `schemas/unc-1.schema.json`, `kats/unc1/unc1_kats.v1.json`, and `docs/vm/OPCODES_NUM.md`.

- Entered through WASM specs (`docs/wasm/spec/*`):
  - Cross-check with `docs/canon/CANON-REFERENCE.md`, `TEST_STRATEGY.md`, and `docs/ops/WASM_RUNTIME_HARDENING_TASKLIST.md`.

## 4) By Job-To-Be-Done

- I need to implement runtime behavior safely:
  - `START-HERE.md`
  - `ARCHITECTURE.md`
  - `docs/canon/CANON-REFERENCE.md`
  - `TEST_STRATEGY.md`
  - `QUALITY_GATE.md`

- I need to reason about canon and determinism:
  - `docs/canon/CANON-REFERENCE.md`
  - `docs/canon/UNC-1.md`
  - `docs/vm/OPCODES_NUM.md`
  - `kats/`

- I need WASM execution contracts:
  - `docs/wasm/spec/README.md`
  - `docs/wasm/spec/WASM_EXECUTION_CHARTER.md`
  - `docs/wasm/spec/WASM_ABI_V1.md`
  - `docs/wasm/spec/WASM_CAPABILITY_MODEL_V1.md`
  - `docs/wasm/spec/WASM_DETERMINISM_PROFILE_V1.md`
  - `docs/wasm/spec/WASM_ATTESTATION_POLICY_V1.md`
  - `docs/wasm/spec/WASM_RECEIPT_BINDING_V1.md`
  - `docs/wasm/spec/WASM_ERROR_CODES_V1.md`

- I need operations and rollout:
  - `docs/ops/TASK_ORCHESTRATION_PROTOCOL.md`
  - `schemas/task.lifecycle.event.v1.json`
  - `docs/ops/FOREVER_BOOTSTRAP.md`
  - `docs/ops/GITEA_SOURCE_FLOW.md`
  - `docs/ops/MCP_RUNTIME_VALIDATION.md`
  - `docs/lifecycle/RELEASE_READINESS.md`

## 5) Status Snapshot (High-Level)

- `START-HERE.md`: Active
- `ARCHITECTURE.md`: Active
- `ARCHITECTURE_v2.md`: Active
- `docs/canon/CANON-REFERENCE.md`: Active
- `docs/canon/UNC-1.md`: Active
- `docs/wasm/spec/README.md`: Draft
- `docs/ops/OFFLINE_RECEIPT_VERIFICATION.md`: Planned

## 6) Surgical Lacunas (Keep Visible)

1. Missing human-readable normative spec for `NRF-1.1`.
2. Missing dedicated semantic spec for `@world`.
3. Missing explicit AI Passport entry in canon reference map.
4. Missing explicit composition section between WASM receipt binding and canonical receipt schema.
5. `WASM_CAPABILITY_MODEL_V1` needs tighter runtime semantics for scoped `fs_read` enforcement and failure mapping.

Track closure in:
- `TASKLIST.md`
- `ARCHITECTURE_v2.md` (`As-Built` / lacunas)
- `docs/canon/CANON-REFERENCE.md`

## 7) Anti-Drift Rule

When changing behavior, update in the same PR:

1. Normative doc(s) (`ARCHITECTURE*`, canon docs, specs)
2. Implementation (`crates/`, `services/`)
3. Evidence (`kats/`, tests, conformance vectors)
4. Index references (`START-HERE.md`, `docs/INDEX.md`, this file)

## 8) Reality and Honesty

UBL-CORE is active work. Some sections are mature, some are still being hardened.
Keep explicit lacunas and next steps visible in `TASKLIST.md` and architecture documents.
