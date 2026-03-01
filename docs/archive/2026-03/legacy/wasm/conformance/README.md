# WASM Conformance Pack

**Status**: Active release-pack baseline (30/70)

This folder defines test vectors for WASM runtime conformance.

## Artifacts

- `VECTOR_SCHEMA_V1.json`: required vector shape
- `COVERAGE_MAP.md`: target matrix and progression
- `vectors/v1/positive/*.vector.json`
- `vectors/v1/negative/*.vector.json`

## Execution

```bash
bash scripts/wasm_conformance.sh --out-dir artifacts/wasm-conformance
bash scripts/wasm_conformance.sh --mode runtime --out-dir artifacts/wasm-conformance
bash scripts/wasm_conformance.sh --mode all --out-dir artifacts/wasm-conformance
```

## Policy

- Vectors must be added before implementation changes.
- Negative vectors are mandatory for every security and determinism rule.
- Runtime error mapping now emits canonical `WASM_*` taxonomy codes natively
  (no compatibility aliases in runtime conformance gate).
- Runtime now validates adapter attestation signature over canonical payload
  (`wasm_sha256`, `abi_version`) using Ed25519 DID trust-anchor checks in TR.
- Trust-anchor can be pinned with `UBL_WASM_TRUST_ANCHOR_DID`; runtime is fail-closed
  if attestation signature/anchor are partially present.

## Runtime Gate Strictness

- Runtime vector execution gate (`WASM-CONF-006`) now runs in strict canonical mode:
  expected conformance `code` must equal observed runtime code exactly.
- No alias fallback is used in `stage_runtime_executes_wasm_conformance_vectors`.
