# SPEC_LINEAGE_PROV_OBS_EP1

Status: materialized
Date: 2026-02-23
Source prompt: 03
Scope: OpenLineage emission in Big, PROV bundle assembly in Small, and OBS automation with fail-closed publish rules.

## Section 1 - OpenLineage in Big (`UBL-0`)

### 1.1 Minimal RunEvent Model

For each execution run, Big emits:
- one `START`
- one terminal `COMPLETE` or `FAIL`

Mandatory fields:
- `eventType`
- `eventTime`
- `run.runId`
- `job.namespace`
- `job.name`
- `producer`

Mandatory datasets:
- `inputs[]`
- `outputs[]`

### 1.2 UBL Custom Facets (`ubl_*`)

Custom facets are required on run events via stable schema URLs:

- `ubl_spec_cid`
- `ubl_snapshot_cid`
- `ubl_policy_cid`
- `ubl_world`
- `ubl_runtime_hash`
- `ubl_adapter_hash`
- `ubl_fuel_used`
- `ubl_latency_ms`
- `ubl_platform_did` (optional when platform driven)
- `ubl_receipt_cid`

Schema URL pattern:
- `https://schemas.ubl.agency/openlineage/ubl/v1/{facet_name}.json`

### 1.3 Emission Rules

`START` emission:
- immediately after `run.request` is accepted and execution starts.

`COMPLETE` emission:
- after result receipt (`WF`) is finalized and persisted.

`FAIL` emission:
- on deterministic denial/failure (`FAIL_FUEL_CAP`, `DENY_NONDETERMINISTIC`, adapter/attestation deny).

Idempotency:
- `runId` is stable per run.
- retries may emit additional events with same `runId`, but terminal state must be single-source final in bundle index.

### 1.4 Persistence and Export

Big writes lineage as append-only NDJSON:
- `./data/episodes/{episode_id}/lineage_big.ndjson`

Big also mirrors summarized lineage events into EventStore using existing event contract (`@type=ubl/event`) so TV and audit can consume via:
- `/v1/events`
- `/v1/events/search`

### 1.5 JSON Examples

`dataset.materialize START`

```json
{
  "eventType": "START",
  "eventTime": "2026-02-23T00:10:00Z",
  "run": { "runId": "run-0001" },
  "job": { "namespace": "ubl.big", "name": "dataset.materialize" },
  "producer": "https://github.com/LogLine-Foundation/UBL-CORE/services/ubl_big_gate",
  "inputs": [{ "namespace": "ubl", "name": "b3:spec-cid" }],
  "outputs": [{ "namespace": "ubl", "name": "b3:snapshot-target" }],
  "runFacets": {
    "ubl_spec_cid": { "_schemaURL": "https://schemas.ubl.agency/openlineage/ubl/v1/ubl_spec_cid.json", "value": "b3:spec-cid" },
    "ubl_world": { "_schemaURL": "https://schemas.ubl.agency/openlineage/ubl/v1/ubl_world.json", "value": "a/lab/t/main" },
    "ubl_runtime_hash": { "_schemaURL": "https://schemas.ubl.agency/openlineage/ubl/v1/ubl_runtime_hash.json", "value": "sha256:runtime" }
  }
}
```

`sim.run COMPLETE`

```json
{
  "eventType": "COMPLETE",
  "eventTime": "2026-02-23T00:10:05Z",
  "run": { "runId": "run-0001" },
  "job": { "namespace": "ubl.big", "name": "sim.run" },
  "producer": "https://github.com/LogLine-Foundation/UBL-CORE/services/ubl_big_gate",
  "inputs": [{ "namespace": "ubl", "name": "b3:snapshot-cid" }],
  "outputs": [{ "namespace": "ubl", "name": "b3:result-cid" }],
  "runFacets": {
    "ubl_snapshot_cid": { "_schemaURL": "https://schemas.ubl.agency/openlineage/ubl/v1/ubl_snapshot_cid.json", "value": "b3:snapshot-cid" },
    "ubl_policy_cid": { "_schemaURL": "https://schemas.ubl.agency/openlineage/ubl/v1/ubl_policy_cid.json", "value": "b3:policy-cid" },
    "ubl_adapter_hash": { "_schemaURL": "https://schemas.ubl.agency/openlineage/ubl/v1/ubl_adapter_hash.json", "value": "sha256:adapter" },
    "ubl_fuel_used": { "_schemaURL": "https://schemas.ubl.agency/openlineage/ubl/v1/ubl_fuel_used.json", "value": 120000 },
    "ubl_latency_ms": { "_schemaURL": "https://schemas.ubl.agency/openlineage/ubl/v1/ubl_latency_ms.json", "value": 84 },
    "ubl_receipt_cid": { "_schemaURL": "https://schemas.ubl.agency/openlineage/ubl/v1/ubl_receipt_cid.json", "value": "b3:receipt-cid" }
  }
}
```

## Section 2 - PROV Bundle Final (`bundle_001.json`) in Small (`ubl-0`)

### 2.1 Bundle Structure

Output file:
- `./data/episodes/{episode_id}/bundle_{episode_id}.json`

Top-level shape:
- `metadata`
- `entities`
- `activities`
- `agents`
- `relations`
- `indexes`

### 2.2 Required Mapping

Entities:
- `dataset.spec`
- `dataset.snapshot`
- `sim.result`
- `episode.video`
- `ledger_small.ndjson`
- `ledger_big.ndjson`
- `lineage_big.ndjson`

Activities:
- `materialize`
- `simulate`
- `evaluate`
- `record`
- `verify`

Agents:
- `did:ubl:small`
- `did:ubl:big`
- `did:ubl:web`
- `did:ubl:mobile`
- `did:ubl:cli`
- `did:ubl:llm:*` from advisory passports

Relations:
- `used`
- `wasGeneratedBy`
- `wasAssociatedWith`
- `wasDerivedFrom`

### 2.3 Provenance-of-Provenance

Bundle must include references to:
- `ledger_small.ndjson` hash
- `ledger_big.ndjson` hash
- `lineage_big.ndjson` hash

This lets verifier validate the provenance substrate itself.

### 2.4 Mandatory Video Inclusion

`episode.video` entity includes:
- file path
- `sha256`
- container (`mkv` or `mp4`)
- start/end timestamps

Required relation:
- activity `record` -> generates `episode.video`.

### 2.5 Required Audit Indexes in Bundle

- `run_id -> receipt_cid[]`
- `@type -> cid[]`
- `platform_did -> run_id[]`

### 2.6 Completeness Calculation

`provenance_completeness = linked_nodes / expected_nodes`

Expected nodes at minimum:
- one spec
- one protocol seal
- one run.request
- one sim.result per run
- one terminal episode decision
- one video entity

If below threshold from governance YAML (`>= 0.99` default):
- force archive reason `LOW_COMPLETENESS`.

### 2.7 Minimal Realistic Example

```json
{
  "metadata": { "episode_id": "001", "status": "ARCHIVED", "reason": "NO_VIDEO" },
  "entities": {
    "dataset_spec": { "cid": "b3:spec" },
    "dataset_snapshot": { "cid": "b3:snap" },
    "sim_result": { "cid": "b3:result" },
    "episode_video": { "sha256": "...", "path": "./data/episodes/001/episodio_001.mp4" }
  },
  "activities": {
    "materialize": { "run_id": "run-0001" },
    "simulate": { "run_id": "run-0001" },
    "record": { "obs_session": "obs-001" }
  },
  "agents": {
    "small": "did:ubl:small",
    "big": "did:ubl:big"
  },
  "relations": [
    { "type": "used", "activity": "simulate", "entity": "dataset_snapshot" },
    { "type": "wasGeneratedBy", "entity": "sim_result", "activity": "simulate" }
  ],
  "indexes": {
    "run_to_receipts": { "run-0001": ["b3:r1", "b3:r2"] },
    "type_to_cids": { "ubl/sim.result": ["b3:result"] },
    "platform_to_runs": { "did:ubl:web": ["run-0001"] }
  }
}
```

## Section 3 - OBS Automation in Small

### 3.1 Integration Contract

Protocol:
- OBS WebSocket v5.

Preflight checks:
- connect/auth success
- output directory writable
- current profile and scene available
- recording state is stopped

If preflight fails:
- `episode.start` denied (`PREFLIGHT_FAILED`).

### 3.2 Trigger Rules

On `ubl/episode.start`:
- send `StartRecord` command
- emit judge event: `judge.tv.record.start`

On `ubl/episode.publish` attempt:
- send `StopRecord`
- find resulting MKV
- remux MKV -> MP4
- hash final video
- emit `ubl/episode.video`
- attach to bundle

### 3.3 MKV and Remux Policy

Policy:
- recording container always `mkv`
- publication container target `mp4`

If remux fails:
- keep MKV
- archive with reason `VIDEO_REMUX_FAIL`

### 3.4 Hashing and Fail-Closed

Rules:
- no video file -> archive `NO_VIDEO`
- no hash -> archive `VIDEO_HASH_MISSING`
- hash mismatch between file and chip -> archive `VIDEO_HASH_MISMATCH`

No exception path for publish.

### 3.5 TV Signals

Minimum TV indicators:
- `REC` on/off
- `JUDGE ONLINE` status
- judge events stream:
  - `judge.passport.deny`
  - `judge.autoseal`
  - `judge.replay.divergence`
  - `judge.publish.archive`

### 3.6 OBS Message Examples

Start:

```json
{ "op": 6, "d": { "requestType": "StartRecord", "requestId": "start-001" } }
```

Stop:

```json
{ "op": 6, "d": { "requestType": "StopRecord", "requestId": "stop-001" } }
```

Resulting `ubl/episode.video` payload:

```json
{
  "@type": "ubl/episode.video",
  "@id": "ep-001-video",
  "@ver": "1.0",
  "@world": "a/lab/t/main",
  "episode_id": "001",
  "container": "mp4",
  "path": "./data/episodes/001/episodio_001.mp4",
  "sha256": "...",
  "started_at": "2026-02-23T00:00:00Z",
  "ended_at": "2026-02-23T00:20:00Z",
  "episode_bundle_cid": "b3:bundle"
}
```

### 3.7 File Naming and Storage

Episode directory:
- `./data/episodes/001/`

Required outputs:
- `episodio_001.mkv`
- `episodio_001.mp4` (if remux succeeds)
- `bundle_001.json`
- `ledger_small.ndjson`
- `ledger_big.ndjson`
- `lineage_big.ndjson`

## Implementation Tasks (Ordered)

1. Implement OpenLineage event emitter in Big execution path.
2. Persist lineage NDJSON per episode and mirror summary events to EventStore.
3. Implement PROV bundle assembler in Small.
4. Add bundle completeness calculator and archive guard.
5. Implement OBS client module with preflight/start/stop/remux/hash.
6. Emit `ubl/episode.video` and wire fail-closed publish rule.

## Files to Create or Change

- `/Users/ubl-ops/UBL-CORE/crates/ubl_lineage/Cargo.toml` (new)
- `/Users/ubl-ops/UBL-CORE/crates/ubl_lineage/src/lib.rs` (new)
- `/Users/ubl-ops/UBL-CORE/crates/ubl_lineage/src/openlineage.rs` (new)
- `/Users/ubl-ops/UBL-CORE/crates/ubl_lineage/src/prov_bundle.rs` (new)
- `/Users/ubl-ops/UBL-CORE/crates/ubl_runtime/src/pipeline/processing.rs`
- `/Users/ubl-ops/UBL-CORE/services/ubl_small_gate/src/obs.rs` (new)
- `/Users/ubl-ops/UBL-CORE/services/ubl_small_gate/src/episode_finalize.rs` (new)
- `/Users/ubl-ops/UBL-CORE/docs/ops/EPISODE_TEMPLATE.md` (optional field sync)
