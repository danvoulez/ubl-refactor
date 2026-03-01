# WASM Capability Model V1

**Status**: Draft

## Default Policy

- Deny by default.
- Explicit allowlist per adapter capability profile.

## Capability Classes

- `compute` (always required)
- `memory` (bounded)
- `clock` (virtualized only)
- `rng` (seeded deterministic only)
- `fs_read` (explicitly scoped; optional)
- `network` (denied in `deterministic_v1`)

## Rejection Rules

- Requested capability not in allowlist -> `WASM_CAPABILITY_DENIED`
- Any network capability under `deterministic_v1` -> `WASM_CAPABILITY_DENIED_NETWORK`
