# SPEC_EPISODE_RUNNER_EP1

Status: materialized
Date: 2026-02-23
Source prompt: 09
Scope: one-command deterministic Episode 1 execution from preflight to verification and final artifacts.

## 1. Interface

### 1.1 Commands

Primary entry:

```bash
just episode-1
```

Equivalent CLI path:

```bash
cargo run -p ubl_episode_runner -- run 001
```

### 1.2 Flags and Env

- `EPISODE_ID` default `001`
- `SEED` optional; if absent runner generates and records
- `RUN_DURATION_SEC` optional
- `REPLAY_SAMPLE` default `0.05`
- `STRICT` default `true`
- `FINALIZE_ONLY` optional
- `VERIFY_ONLY` optional

### 1.3 Required Output Files

Under `./data/episodes/{episode_id}/`:

- `episodio_{id}.mkv`
- `episodio_{id}.mp4` (if remux succeeds)
- `bundle_{id}.json`
- `ledger_small.ndjson`
- `ledger_big.ndjson`
- `lineage_big.ndjson`
- `report.md`
- `report.json`
- `runner_state.json`

## 2. Orchestration Sequence

### 2.1 INIT and Infra Boot

1. ensure output directory exists
2. ensure `small` and `big` health endpoints respond
3. ensure TV endpoint responds
4. persist initial runner state

### 2.2 PREFLIGHT (Fail-Closed)

Checks:
- governance YAML loads and validates
- storage paths writable (`cas`, `sqlite`, `ledgers`, episode dir)
- attestation assets available
- OBS websocket reachable
- verifier binary available

On preflight failure:
- stop flow
- write `preflight_fail.md` + state reason
- final status `ARCHIVED/PREFLIGHT_FAILED`

### 2.3 APITO (`ubl/episode.start`)

Runner submits `ubl/episode.start` to Small.
Small responsibilities triggered:
- start OBS recording
- transition to `DRAFT`
- initialize method dossier

Runner stores start receipt CID in state.

### 2.4 Method Build and Seal

Runner waits for:
- proposal/advisory cycle completion
- quorum satisfied or autoseal reached
- `ubl/protocol.seal` receipt available

Timeout behavior:
- if `max_draft_minutes` exceeded and no quorum -> archive `NO_QUORUM`.

### 2.5 RUNNING Phase

1. submit `ubl/run.request` from Small to Big
2. start `web_gen`, `mobile_sim`, `cli_batch`
3. monitor event stream and KPIs
4. stop when batch complete or duration reached

Monitored errors:
- signature invalid
- replay
- rate limit
- fuel cap fails

### 2.6 Publish and Finalization

Runner requests `ubl/episode.publish` on Small.
Small executes:
- stop OBS
- remux
- hash video
- emit `ubl/episode.video`
- assemble bundle
- export ledgers and lineage

If video/hash missing:
- force archive (`NO_VIDEO` or `VIDEO_HASH_MISSING`).

### 2.7 Independent Verification

Run verifier:

```bash
ubl-verify episode ./data/episodes/{id}/ --strict --replay-sample {REPLAY_SAMPLE}
```

If verifier fails:
- final status forced to `ARCHIVED/VERIFY_FAIL`.

## 3. APIs Used

Small (minimum):
- `GET /healthz`
- `POST /v1/chips` for episode control chips
- `GET /v1/events/search`
- `GET /v1/events`

Big (minimum):
- `GET /healthz`
- `POST /v1/chips` for platform events and run artifacts
- `GET /v1/events/search`

TV:
- `GET /tv`
- `GET /tv/stream`

## 4. Runner State Machine

States:

- `INIT`
- `PREFLIGHT`
- `STARTED`
- `METHOD_SEALED`
- `RUNNING`
- `FINALIZING`
- `VERIFYING`
- `DONE`

Each transition writes `runner_state.json` with timestamp and reason.

## 5. Observability

Runner emits structured logs with fields:
- `episode_id`
- `state`
- `step`
- `duration_ms`
- `receipt_cid` (when available)
- `error_code` (on failures)

Judge events emitted by runner integration path:
- `judge.preflight.ok|fail`
- `judge.episode.start`
- `judge.autoseal`
- `judge.publish.ok|archive`

## 6. Idempotency and Re-Run Policy

If target episode dir exists:
- default behavior: abort with `EPISODE_DIR_EXISTS`.
- optional `--suffix` creates `001a`, `001b`.

Modes:
- `FINALIZE_ONLY`: skip run generation, perform publish/export from existing state.
- `VERIFY_ONLY`: run verifier only against existing artifact set.

## 7. Acceptance Criteria

- one command executes complete flow and produces expected artifacts.
- deterministic seed produces repeatable non-clock-dependent outputs.
- predictable failures archive with explicit reasons.
- verifier output always generated and linked to episode state.

## 8. Repository Layout

Suggested additions:

- `/Users/ubl-ops/UBL-CORE/crates/ubl_episode_runner/Cargo.toml`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_episode_runner/src/main.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_episode_runner/src/state.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_episode_runner/src/preflight.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_episode_runner/src/orchestrate.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_episode_runner/src/finalize.rs`
- `/Users/ubl-ops/UBL-CORE/justfile` (episode recipes)
- `/Users/ubl-ops/UBL-CORE/ops/episode1/runner.env`

## Implementation Order

1. Create runner crate and state machine skeleton.
2. Implement preflight checks and fail-closed exits.
3. Implement start -> seal wait loop.
4. Implement run orchestration and mock actor process management.
5. Implement finalize/export/video hooks.
6. Integrate verifier execution and status override.
7. Wire `just episode-1` recipe.
