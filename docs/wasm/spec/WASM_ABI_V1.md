# WASM ABI V1

**Status**: Draft

## Contract

- ABI identifier must be explicit (`abi_version`).
- Input payload must be canonical bytes with explicit content type.
- Output payload must include canonical bytes and content type.
- Runtime must reject unknown ABI versions with fail-closed behavior.

## Required Input Fields

- `abi_version`
- `input_cid`
- `content_type`
- `payload_bytes_b64`

## Required Output Fields

- `output_cid`
- `content_type`
- `payload_bytes_b64`
- `execution_meta`

## Error Mapping

- Missing `abi_version` -> `WASM_ABI_MISSING_VERSION`
- Unsupported `abi_version` -> `WASM_ABI_UNSUPPORTED_VERSION`
- Malformed payload -> `WASM_ABI_INVALID_PAYLOAD`
