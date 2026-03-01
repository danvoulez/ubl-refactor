# WASM Determinism Profile V1 (`deterministic_v1`)

**Status**: Draft

## Determinism Rules

- Time source is virtualized and deterministic.
- RNG is seeded and deterministic.
- Locale-sensitive behavior is fixed.
- Floating-point behavior follows profile constraints and must be replay-stable.
- Nondeterministic host calls are denied.

## Replay Requirement

Given identical input and profile, output CID and receipt claims must match.

## Failure Code

Replay drift -> `WASM_DETERMINISM_VIOLATION`.
