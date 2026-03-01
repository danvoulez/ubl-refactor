# WASM Attestation Policy V1

**Status**: Draft

## Integrity Requirements

- Module hash must match declared hash.
- Signer trust anchor must be pinned outside mutable release assets.
- Unsupported signature algorithm must fail closed.

## Minimum Claims

- `module_sha256`
- `attestation_signer_id`
- `attestation_algorithm`

## Failure Codes

- Hash mismatch -> `WASM_VERIFY_HASH_MISMATCH`
- Signature invalid -> `WASM_VERIFY_SIGNATURE_INVALID`
- Trust anchor mismatch -> `WASM_VERIFY_TRUST_ANCHOR_MISMATCH`
