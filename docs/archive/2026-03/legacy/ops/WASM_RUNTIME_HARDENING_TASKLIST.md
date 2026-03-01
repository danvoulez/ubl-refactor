# UBL-CORE — WASM Runtime Hardening Tasklist

**Status**: Active execution source of truth (WASM hardening program)
**Date**: 2026-02-23
**Objective**: Raise UBL WASM execution from PoC-level integration to production-grade deterministic runtime.

---

## Operating Rules (Non-Negotiable)

- Spec-first: no behavior change before spec update.
- Vectors-first: no implementation without conformance vectors.
- Red-first evidence required for bug fixes and contract changes.
- Fail-closed for integrity, attestation, and capability checks.
- Receipts are mandatory proof; no side channel as source of truth.

---

## Program Gates

- `G0`: Baseline and charter approved.
- `G1`: Spec pack V1 complete and reviewed.
- `G2`: Conformance vectors and coverage map approved.
- `G3`: Conformance harness + CI gates active.
- `G4`: Security/determinism defects closed.
- `G5`: Release-readiness and DoD complete.

---

## Phase 0 — Baseline and Charter (`G0`)

- [ ] Capture baseline commit, current wasm execution paths, and known gaps.
- [ ] Publish scope boundary (in-scope / out-of-scope).
- [ ] Publish program charter and acceptance criteria.
- [ ] Approve deterministic profile name (`deterministic_v1`) and governance owner.

**Exit evidence**
- `docs/wasm/spec/WASM_EXECUTION_CHARTER.md`
- baseline evidence artifact under `artifacts/wasm-conformance/`

---

## Phase 1 — Spec Pack V1 (`G1`)

- [ ] Finalize ABI contract (`WASM_ABI_V1.md`).
- [ ] Finalize capability model (`WASM_CAPABILITY_MODEL_V1.md`).
- [ ] Finalize determinism profile (`WASM_DETERMINISM_PROFILE_V1.md`).
- [ ] Finalize attestation/integrity policy (`WASM_ATTESTATION_POLICY_V1.md`).
- [ ] Finalize receipt binding claims (`WASM_RECEIPT_BINDING_V1.md`).
- [ ] Finalize error taxonomy and mapping (`WASM_ERROR_CODES_V1.md`).

**Exit evidence**
- `docs/wasm/spec/README.md`
- all V1 spec files above marked `Status: Active`.

---

## Phase 2 — Conformance Vectors First (`G2`)

- [x] Approve vector schema (`VECTOR_SCHEMA_V1.json`).
- [x] Approve coverage map (`COVERAGE_MAP.md`).
- [x] Seed initial positive/negative vectors and validate schema.
- [x] Expand to first hard gate set: >= 20 positive and >= 20 negative.
- [x] Expand to release target set: >= 30 positive and >= 70 negative.

**Minimum negative categories**
- ABI violations
- Integrity/attestation failures
- Capability denials
- Determinism violations
- Resource guards (fuel/time/memory)
- Receipt claim binding failures

**Exit evidence**
- `docs/wasm/conformance/README.md`
- `docs/wasm/conformance/vectors/v1/{positive,negative}`
- conformance report under `artifacts/wasm-conformance/`

---

## Phase 3 — Harness and CI Gates (`G3`)

- [x] Implement deterministic conformance runner (`scripts/wasm_conformance.sh`).
- [x] Emit canonical report (`latest.json`, `latest.md`).
- [ ] Add CI workflow stage for WASM conformance.
- [ ] Enforce fail-on-regression for gate set.

**Exit evidence**
- CI run with green WASM conformance gate.
- report artifact attached to CI run.

---

## Phase 4 — Implementation Hardening Loop (`G4`)

- [x] Run suite and classify failures by domain.
- [x] Fix integrity/attestation defects first.
- [x] Fix capability isolation defects.
- [ ] Fix determinism drift defects.
- [x] Fix resource guard defects.
- [x] Fix receipt binding and error mapping defects.
- [ ] Re-run full suite after each domain closure.

**Closure rule**
- No issue is closed without an added vector proving regression safety.

---

## Phase 5 — Release Readiness (`G5`)

- [ ] Three consecutive clean conformance runs.
- [ ] Deterministic replay pass (`0` divergence on required set).
- [ ] Security review checklist signed.
- [ ] Performance sanity within budget.
- [ ] Docs + runbook + incident response updated.

---

## Definition of Done (DoD)

1. Spec pack V1 approved and versioned.
2. Vector suite at release target size (`>=30` positive, `>=70` negative).
3. Conformance runner active in CI and failing correctly on regression.
4. Determinism replay gate reports zero divergence for required corpus.
5. Integrity/attestation path is fail-closed and covered by negative vectors.
6. Receipt includes required WASM binding claims.
7. Error taxonomy is stable and fully mapped in tests.
8. Security and operations signoff recorded.

---

## Deferred / Out of Scope for This Program

- Full runtime rewrite to WASM-only architecture.
- Product-shell features unrelated to wasm execution hardening.
- Non-deterministic capability extensions.
