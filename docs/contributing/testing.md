# Testing Strategy

Testing in UBL-CORE combines deterministic vectors, contract checks, and unit/integration coverage.

## Minimum local test bar

```bash
cargo test --workspace --lib
cargo test --workspace --test '*'
bash scripts/contract_suite.sh --out-dir artifacts/contract
bash scripts/conformance_suite.sh --out-dir artifacts/conformance
```

## Principles

- Behavior changes must add or update tests/vectors.
- Deterministic behavior must include reproducible assertions.
- Regressions should be captured with a red-first test when feasible.
