# Stage secret rotation (runtime state, no env mutation)

The pipeline keeps stage-auth secrets in runtime state and persists them in durable storage.

## Behavior

- On startup, `PipelineConfig` loads `stage_secret_current` and optional `stage_secret_prev`.
- If durable storage has a persisted row, it overrides the startup values.
- If no current secret is configured, pipeline derives one from the signing key (dev-safe fallback).
- Receipt auth tokens are computed and verified using injected secrets from pipeline state.
- During `ubl/key.rotate`, pipeline updates in-memory secrets (`current <- new`, `prev <- old`) and persists both values atomically via `DurableStore`.

## Operational note

`std::env::set_var` is intentionally not used in the production rotation path anymore.
Secrets must be provided at startup config/env boundaries, then flow through pipeline config/state.
