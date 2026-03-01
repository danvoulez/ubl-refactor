# WASM Receipt Binding V1

**Status**: Draft

## Required Claims in Receipt

- `wasm.module.sha256`
- `wasm.abi.version`
- `wasm.profile`
- `wasm.fuel.used`
- `wasm.memory.max_bytes`
- `wasm.verify.status`

## Rule

Any missing required claim is a conformance failure.

## Failure Code

Missing required receipt claim -> `WASM_RECEIPT_BINDING_MISSING_CLAIM`.
