# UBL-CORE Quality Gate

**Status**: active  
**Owner**: Core Runtime + Ops  
**Last reviewed**: 2026-02-22

This is the release and merge gate policy for UBL-CORE.

## Gate Levels

1. G0 Local (developer machine)
- Must pass before opening PR:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `bash scripts/contract_suite.sh --out-dir artifacts/contract`
  - `cargo test --workspace --lib`
  - `bash scripts/conformance_suite.sh --out-dir artifacts/conformance`

2. G1 PR CI
- Required jobs:
  - `WA` (format/build/lint)
  - `CONTRACT`
  - `TR` (unit/integration/doc)
  - `CONFORMANCE`
  - `EXECUTE` (security checks)
  - Final `WF` gate job must pass

3. G2 Release Readiness
- All `G1` green on target commit.
- Evidence artifacts captured and retained.
- Release governance checklist green (`docs/lifecycle/RELEASE_READINESS.md`).

## Mandatory Evidence

Every behavior-changing PR must include:

1. `RED-FIRST EVIDENCE`:
- the failing command/output from contract/regression test before fix.

2. `GREEN EVIDENCE`:
- the passing command/output after fix.

3. `CONFORMANCE`:
- summary of conformance run or CI artifact link.

## Blocking Conditions (No Merge)

1. Any failing contract/conformance rule.
2. Public behavior change without contract test/vector update.
3. Regression fix without a reproducing pre-fix failing test.
4. Mismatch between code change and test evidence in PR description.

## Exceptions

No silent exceptions.  
Any exception must be explicitly documented in PR and approved by maintainers.
