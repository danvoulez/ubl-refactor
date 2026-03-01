# UBL-CORE Test Strategy

**Status**: active  
**Owner**: Core Runtime  
**Last reviewed**: 2026-02-22

This document defines how UBL-CORE tests are designed so we do not create test suites that only mirror the latest implementation.

## Core Rule

Tests validate contracts, not code structure.

## Test Layers

1. Contract tests (black-box, pre-implementation)
- Target: external behavior and invariants.
- Must be written first (expected fail before implementation).
- Primary suites:
  - `crates/ubl_runtime/tests/knock_vector_matrix.rs`
  - `crates/rb_vm/tests/rho_contract_vectors.rs`
  - `crates/ubl_runtime/tests/canon_guardrails.rs`

2. Implementation tests (unit/integration)
- Target: internal module correctness and regressions.
- Can evolve with refactors if public behavior is unchanged.
- Typical locations:
  - `crates/*/src/*` inline tests
  - `crates/*/tests/*`

3. Conformance tests (standard compatibility)
- Target: stable compatibility contract across implementations and versions.
- Run via `scripts/conformance_suite.sh`.
- Output is a canonical report artifact (`JSON + Markdown + logs`).

4. WASM conformance vectors (determinism and sandbox contract)
- Target: ABI, capability, integrity, resource-guard, and receipt-binding compliance.
- Run via `scripts/wasm_conformance.sh`.
- Vector source: `docs/wasm/conformance/vectors/v1/`.
- Output is a canonical report artifact (`JSON + Markdown`).

## Required Workflow (No Exceptions)

1. Define contract or vector first.
2. Write a failing contract test (`red`).
3. Implement minimal code to pass (`green`).
4. Refactor while keeping contract tests green.
5. Add/adjust implementation tests only after contract is locked.

## Change-Type Requirements

1. Any public behavior change:
- Add/modify at least one contract test or vector before implementation.

2. Bug fix:
- Add a failing regression test that reproduces the bug first.

3. Canonicalization/numerics/crypto change:
- Add or update deterministic vectors in KAT/contract suites.
- Update conformance coverage if behavior surface changed.

4. Error mapping/status change:
- Add contract coverage for code + HTTP + MCP mapping behavior.

## Prohibited Patterns

1. Writing tests that assert private implementation details as the main validation.
2. Changing vectors only to match newly introduced behavior without contract review.
3. Merging behavior changes without explicit contract coverage.

## Evidence in PR

Each PR changing behavior must include:

1. `RED-FIRST EVIDENCE`: command/output proving at least one target test failed before the fix.
2. `GREEN EVIDENCE`: command/output after implementation.
3. Conformance result summary when contract surface changed.

## Execution Commands

```bash
bash scripts/contract_suite.sh --out-dir artifacts/contract
bash scripts/conformance_suite.sh --out-dir artifacts/conformance
bash scripts/wasm_conformance.sh --out-dir artifacts/wasm-conformance
```
