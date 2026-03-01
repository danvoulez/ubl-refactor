# WASM Conformance Coverage Map

## Program Targets

- Gate set (Phase 2): >= 30 positive, >= 70 negative
- Release set (DoD): >= 30 positive, >= 70 negative

## Category Matrix

| Category | Positive target | Negative target | Seed present |
|---|---:|---:|---:|
| abi | 5 | 10 | yes |
| verify | 5 | 15 | yes |
| capability | 5 | 15 | yes |
| determinism | 5 | 10 | yes |
| resource | 5 | 10 | yes |
| receipt_binding | 5 | 10 | yes |

## Seed Counts (Current)

- positive: 30
- negative: 70

## Next Expansion

1. Increase semantic diversity of vectors per category (not only count growth).
2. Add attestation trust-anchor rotation vectors and failure-path replay vectors.
3. Keep strict runtime code equality gate with no alias fallback.
