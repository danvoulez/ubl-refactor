# SPEC_PLATFORM_MOCKS_EP1

Status: materialized
Date: 2026-02-23
Source prompt: 04
Scope: deterministic web/mobile/cli platform actors sending signed events to a single Big data plane, with Small observing and governing.

## 1. Common Platform Event Contract

### 1.1 Types

- `ubl/platform.event.web`
- `ubl/platform.event.mobile`
- `ubl/platform.event.cli`

### 1.2 Required Fields

All platform events must contain:

- `@type`
- `@id`
- `@world`
- `@ver`
- `platform_did`
- `event_time` (UTC)
- `payload`
- `signature`
- `kid`
- `episode_id`
- optional `run_id`

### 1.3 Determinism Rules

- no floating-point values in payload; use integer or fixed decimal strings.
- canonical key ordering through existing UBL canonicalization.
- all generators use seeded RNG.
- default event clock for mocks is derived from episode start + tick offset.

### 1.4 Big-Side Validation

Big validates in CHECK:

- schema validity
- DID signature validity
- replay protection (`nonce` or per-platform sequence)
- rate limits by `platform_did`

HTTP response mapping:

- `400` schema invalid
- `401` signature invalid
- `409` replay/nonce conflict
- `429` rate limit

## 2. Web Generator

### 2.1 Function

- generate deterministic click/session stream.
- optional burst mode for stress.

### 2.2 Implementation

Binary:
- `crates/ubl_platform_mocks/src/bin/web_gen.rs`

Env:
- `BIG_URL`
- `WORLD`
- `PLATFORM_DID=did:ubl:web`
- `PLATFORM_KID`
- `PLATFORM_SIGNING_KEY_HEX`
- `EPISODE_ID`
- `SEED`
- `RATE_PER_SEC` (default 5)
- `BURST_SIZE` (default 20)
- `DURATION_SEC` (default 300)

### 2.3 Event Kinds

- `session.start`
- `click`
- `session.heartbeat`
- `session.end`

Acceptance:
- 100 emitted -> 100 accepted -> receipts visible in EventStore stream.

## 3. Mobile Simulator

### 3.1 Function

- deterministic “real world style” telemetry:
  - coarse location cell id
  - acceleration bucket
  - battery percent
  - network type enum

### 3.2 Profiles

- `idle`
- `walking`
- `driving`

### 3.3 Implementation

Binary:
- `crates/ubl_platform_mocks/src/bin/mobile_sim.rs`

Env:
- `SCENARIO=idle|walking|driving`
- shared env from web generator

Determinism:
- no host wall-clock randomness
- event_time derived from episode tick

Acceptance:
- all 3 scenarios run and produce coherent traces accepted by Big.

## 4. CLI Batch Generator

### 4.1 Function

- generate large deterministic load plan and execute with bounded concurrency.

### 4.2 Algorithm

RNG:
- `xoshiro256++` (versioned as `xoshiro256pp-v1`).

Distribution:
- default uniform; optional zipf for skew tests.

### 4.3 Backpressure and Retry

- `WORKERS` default `8`
- idempotent retries with same `@id`
- max retry `3`
- jitter disabled (determinism)

### 4.4 Scale Target

- 10k steps without dropping Big availability.

Acceptance:
- p95 latency and fuel metrics available in event stream and dashboard.

## 5. Integration with Small TV/Judge

Small consumes stream and renders:

- accepted/denied counters per platform
- events/s
- top error codes (`401`, `409`, `429`)
- optional run correlation heatmap

Mandatory judge events surfaced:
- `judge.passport.deny`
- `judge.autoseal`
- `judge.replay.divergence`
- `judge.publish.archive`

Show mode:
- pulse effect when Big processes run terminal events.

## 6. Local Infra-0 Setup

Compose services:

- `small`
- `big`
- `tv`
- `web_gen`
- `mobile_sim`
- `cli_batch`
- optional `verifier`

Directories:

- `./data/episodes/001/`
- `./data/cas/`
- `./data/sqlite/`
- `./data/ledgers/`

Commands:

- `just up`
- `just episode-1`
- `just down`
- `just nuke`

## 7. Test Checklist

- signature valid/invalid path
- deterministic replay of first 100 events by same seed
- replay rejection on duplicate sequence
- rate limit enforcement by DID
- restart resilience with no evidence loss
- OBS off path leads to archive (`NO_VIDEO`)

## Files to Create or Change

- `/Users/ubl-ops/UBL-CORE/crates/ubl_platform_mocks/Cargo.toml` (new)
- `/Users/ubl-ops/UBL-CORE/crates/ubl_platform_mocks/src/lib.rs` (new)
- `/Users/ubl-ops/UBL-CORE/crates/ubl_platform_mocks/src/bin/web_gen.rs` (new)
- `/Users/ubl-ops/UBL-CORE/crates/ubl_platform_mocks/src/bin/mobile_sim.rs` (new)
- `/Users/ubl-ops/UBL-CORE/crates/ubl_platform_mocks/src/bin/cli_batch.rs` (new)
- `/Users/ubl-ops/UBL-CORE/crates/ubl_runtime/src/pipeline/check/` (platform event validators)
- `/Users/ubl-ops/UBL-CORE/ops/episode1/platform_mocks.env` (new)
- `/Users/ubl-ops/UBL-CORE/ops/monitoring/docker-compose.yml`
