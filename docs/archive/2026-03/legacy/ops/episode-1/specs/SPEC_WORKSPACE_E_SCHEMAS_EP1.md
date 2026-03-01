# SPEC_WORKSPACE_E_SCHEMAS_EP1

Status: materialized
Date: 2026-02-23
Source prompt: 02
Scope: split the monorepo into executable Small/Big profiles and define strict Episode 1 contracts.

## Parte 1 - Workspace and Profiles

### 1. Strategy Decision

Decision: two service binaries plus shared runtime crates.

- Keep core crates shared.
- Introduce two thin services:
  - `services/ubl_small_gate`
  - `services/ubl_big_gate`
- Keep `services/ubl_gate` as compatibility wrapper during migration window.

Why this choice:

- Better operational safety than env-only role switching.
- Avoid accidental exposure of UI/MCP surfaces on Big.
- Cleaner CI matrix (`small` and `big` build/test independently).

### 2. Recommended Repository Layout

```text
/Users/ubl-ops/UBL-CORE/
  crates/
    ubl_runtime/
    ubl_eventstore/
    ubl_receipt/
    ubl_chipstore/
    ubl_types/
    ubl_cli/
    ubl_governance/          # new
    ubl_schemas/             # new
    ubl_lineage/             # new (OpenLineage + PROV helpers)
    ubl_episode_runner/      # new
    ubl_verifier/            # new
  services/
    ubl_gate/                # compatibility wrapper (transitional)
    ubl_small_gate/          # new
    ubl_big_gate/            # new
  ops/
    lab.governance.v0.yaml   # new
  schemas/
    episode1/                # new
```

### 3. Crate and Module Matrix (small vs big)

| Crate/Module | Small | Big | Rationale |
|---|---:|---:|---|
| `crates/ubl_runtime` | yes | yes | Shared deterministic pipeline core. |
| `crates/ubl_eventstore` | yes | yes | Audit and timeline evidence in both planes. |
| `crates/ubl_chipstore` | yes | yes | CAS + chip persistence on both. |
| `crates/ubl_receipt` | yes | yes | Unified receipt model and verification. |
| `crates/ubl_types` | yes | yes | Common payload contracts. |
| `crates/ubl_did` + `crates/ubl_kms` | yes | yes | Identity/signing for ingress and issuance. |
| `crates/ubl_governance` (new) | yes | read-subset | Full judge on Small; strict subset on Big. |
| `crates/ubl_schemas` (new) | yes | yes | Fail-closed type validation in CHECK. |
| `crates/ubl_lineage` (new) | aggregate | emit | Big emits lineage, Small aggregates for bundle. |
| `crates/ubl_verifier` (new) | tool | tool | Independent verifier binary; not request-path. |
| `services/ubl_small_gate` (new) | yes | no | Judge, TV/OBS, publication decisions. |
| `services/ubl_big_gate` (new) | no | yes | Data plane execution and ingest. |
| UI routes (`/console`, `/registry`, `/audit`, `/ui/_llm`) | yes | no | Human and governance layer only on Small. |
| MCP routes (`/mcp/*`) | optional | no | Tooling/control should terminate at Small. |

### 4. Features and `cfg` Plan

Common feature flags:

- `small`: enable judge state machine, episode endpoints, TV adapters.
- `big`: enable heavy execution surfaces, lineage emitter.
- `tv`: server-side TV routes and SSE.
- `obs`: OBS websocket automation client.
- `wasm`: WASM adapter execution in TR.
- `attest`: attestation verification and fail-closed checks.
- `lineage`: OpenLineage emission and persistence.
- `prov`: PROV bundle assembly.

Build rules:

- `ubl_small_gate`: default features `small,tv,obs,attest,prov`.
- `ubl_big_gate`: default features `big,wasm,attest,lineage`.
- `ubl_big_gate` must not compile `tv` or `obs`.

### 5. Environment Variables and Defaults by Profile

Common:

- `UBL_GOVERNANCE_YAML=./ops/lab.governance.v0.yaml`
- `UBL_STORE_BACKEND=sqlite`
- `UBL_STORE_DSN=file:./data/ubl.db?mode=rwc&_journal_mode=WAL`
- `UBL_EVENTSTORE_DSN=file:./data/events.db?mode=rwc&_journal_mode=WAL`

Small defaults:

- `UBL_ROLE=small`
- `UBL_SMALL_BIND=0.0.0.0:4000`
- `UBL_BIG_URL=http://127.0.0.1:5000`
- `OBS_WS_URL=ws://127.0.0.1:4455`
- `UBL_ENABLE_REAL_LLM=false`
- `UBL_TV_SSE_PATH=/tv/stream`

Big defaults:

- `UBL_ROLE=big`
- `UBL_BIG_BIND=0.0.0.0:5000`
- `UBL_EXECUTION_PROFILE=deterministic_v1`
- `UBL_FUEL_CAP=500000`
- `UBL_ATTEST_DIR=./security/attestations`
- `UBL_COSIGN_VERIFY_MODE=required`

Storage defaults:

- `UBL_CAS_ROOT=./data/cas`
- `UBL_SQLITE_PATH=./data/sqlite/ubl.db`
- `UBL_LEDGER_PATH=./data/ledgers`

### 6. Build, Test, Run Commands

Build:

```bash
cargo build -p ubl_small_gate
cargo build -p ubl_big_gate
```

Tests:

```bash
cargo test -p ubl_runtime --features small
cargo test -p ubl_runtime --features big
cargo test -p ubl_schemas
cargo test -p ubl_governance
```

Run local:

```bash
cargo run -p ubl_small_gate
cargo run -p ubl_big_gate
```

Compose/just:

```bash
just up
just episode-1
just down
```

## Parte 2 - Schemas and Data Contracts (Fail-Closed)

### 7. Common Envelope Schema

Canonical fields for every Episode 1 type:

- `@type` string
- `@id` string
- `@world` string matching `a/{app}/t/{tenant}`
- `@ver` string

Global rejection rules:

- Missing canonical fields: `DENY_SCHEMA_REQUIRED_FIELD`
- Invalid `@world`: `DENY_WORLD_FORMAT`
- Unknown `@type`: `DENY_TYPE_UNSUPPORTED`
- Unsupported `@ver`: `DENY_SCHEMA_VERSION`

### 8. Contract Blocks by Type

Each type section below defines required fields, validation rules, minimal valid example, and deny errors.

#### 8.1 `ubl/episode.start`

Required fields:
- `episode_id`, `seed`, `policy_cid`, `requested_by_did`

Rules:
- `seed` must be uint64.
- `policy_cid` must be CID format.

Example:
```json
{"@type":"ubl/episode.start","@id":"ep-001-start","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","seed":42,"policy_cid":"b3:abc","requested_by_did":"did:ubl:small"}
```

Deny:
- `DENY_EPISODE_SEED_INVALID`
- `DENY_POLICY_CID_INVALID`

#### 8.2 `ubl/episode.publish`

Required fields:
- `episode_id`, `decision`, `reason`

Rules:
- `decision` in `PUBLISHED|ARCHIVED`.
- If `decision=PUBLISHED`, `reason` must be `OK`.

Example:
```json
{"@type":"ubl/episode.publish","@id":"ep-001-publish","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","decision":"ARCHIVED","reason":"NO_VIDEO"}
```

Deny:
- `DENY_DECISION_INVALID`
- `DENY_REASON_INVALID`

#### 8.3 `ubl/episode.bundle`

Required fields:
- `episode_id`, `bundle_cid`, `lineage_path`, `ledger_small_path`, `ledger_big_path`

Rules:
- `bundle_cid` must resolve to generated bundle object.

Example:
```json
{"@type":"ubl/episode.bundle","@id":"ep-001-bundle","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","bundle_cid":"b3:def","lineage_path":"./data/episodes/001/lineage_big.ndjson","ledger_small_path":"./data/episodes/001/ledger_small.ndjson","ledger_big_path":"./data/episodes/001/ledger_big.ndjson"}
```

Deny:
- `DENY_BUNDLE_CID_INVALID`
- `DENY_BUNDLE_PATH_MISSING`

#### 8.4 `ubl/episode.video`

Required fields:
- `episode_id`, `sha256`, `container`, `path`, `started_at`, `ended_at`, `episode_bundle_cid`

Rules:
- `container` in `mkv|mp4`.
- `sha256` hex length 64.

Example:
```json
{"@type":"ubl/episode.video","@id":"ep-001-video","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","container":"mp4","path":"./data/episodes/001/episodio_001.mp4","started_at":"2026-02-23T00:00:00Z","ended_at":"2026-02-23T00:20:00Z","episode_bundle_cid":"b3:def"}
```

Deny:
- `DENY_VIDEO_HASH_INVALID`
- `DENY_VIDEO_CONTAINER_INVALID`

#### 8.5 `ubl/dataset.spec.proposal`

Required fields:
- `episode_id`, `author_did`, `proposal_hash`, `content_hash`

Rules:
- `author_did` must be in committee allowlist.

Example:
```json
{"@type":"ubl/dataset.spec.proposal","@id":"ep-001-spec-prop-1","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","author_did":"did:ubl:llm:gpt","proposal_hash":"b3:123","content_hash":"b3:124"}
```

Deny:
- `DENY_PROPOSAL_AUTHOR_UNAUTHORIZED`

#### 8.6 `ubl/dataset.spec`

Required fields:
- `episode_id`, `spec_cid`, `seed`, `generator_version`

Rules:
- `generator_version` must be semver.

Example:
```json
{"@type":"ubl/dataset.spec","@id":"ep-001-spec","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","spec_cid":"b3:200","seed":42,"generator_version":"1.0.0"}
```

Deny:
- `DENY_SPEC_INVALID`

#### 8.7 `ubl/protocol.seal`

Required fields:
- `episode_id`, `method_spec_cid`, `sealed_by_did`, `sealed_at`

Rules:
- Only Small DID may seal.

Example:
```json
{"@type":"ubl/protocol.seal","@id":"ep-001-seal","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","method_spec_cid":"b3:200","sealed_by_did":"did:ubl:small","sealed_at":"2026-02-23T00:05:00Z"}
```

Deny:
- `DENY_SEAL_UNAUTHORIZED`

#### 8.8 `ubl/run.request`

Required fields:
- `episode_id`, `run_id`, `spec_cid`, `policy_cid`, `execution_profile`, `budget`, `allowed_adapter_hashes`

Rules:
- `execution_profile` must be `deterministic_v1`.
- `budget.fuel_cap` must be positive.

Example:
```json
{"@type":"ubl/run.request","@id":"ep-001-run-0001","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","run_id":"run-0001","spec_cid":"b3:200","policy_cid":"b3:abc","execution_profile":"deterministic_v1","budget":{"fuel_cap":500000},"allowed_adapter_hashes":["sha256:aaa"]}
```

Deny:
- `DENY_EXEC_PROFILE_INVALID`
- `DENY_ADAPTER_NOT_ALLOWLISTED`

#### 8.9 `ubl/advisory`

Required fields:
- `episode_id`, `passport_cid`, `model_identity`, `prompt_hash`, `input_hash`, `advisory_hash`

Rules:
- Passport must be valid and unexpired.

Example:
```json
{"@type":"ubl/advisory","@id":"ep-001-adv-1","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","passport_cid":"b3:301","model_identity":{"family":"gpt","version":"4o"},"prompt_hash":"b3:302","input_hash":"b3:303","advisory_hash":"b3:304"}
```

Deny:
- `DENY_PASSPORT_INVALID`
- `DENY_PASSPORT_EXPIRED`

#### 8.10 `ubl/advisory.bundle`

Required fields:
- `episode_id`, `advisory_cids`, `quorum_summary`

Rules:
- At least 2 advisory entries from 2 distinct model families.

Example:
```json
{"@type":"ubl/advisory.bundle","@id":"ep-001-adv-bundle","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","advisory_cids":["b3:401","b3:402"],"quorum_summary":{"approvals":2,"pool":3,"diversity_key":"model_family"}}
```

Deny:
- `DENY_QUORUM_NOT_MET`
- `DENY_DIVERSITY_NOT_MET`

#### 8.11 `ubl/dataset.materialize`

Required fields:
- `episode_id`, `run_id`, `spec_cid`, `snapshot_target`

Rules:
- `snapshot_target` must be writable path under episode dir.

Example:
```json
{"@type":"ubl/dataset.materialize","@id":"ep-001-mat-1","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","run_id":"run-0001","spec_cid":"b3:200","snapshot_target":"./data/episodes/001/snapshot-0001.json"}
```

Deny:
- `DENY_MATERIALIZE_TARGET_INVALID`

#### 8.12 `ubl/dataset.snapshot`

Required fields:
- `episode_id`, `run_id`, `snapshot_cid`, `record_count`

Rules:
- `record_count` must be non-negative integer.

Example:
```json
{"@type":"ubl/dataset.snapshot","@id":"ep-001-snap-1","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","run_id":"run-0001","snapshot_cid":"b3:500","record_count":10000}
```

Deny:
- `DENY_SNAPSHOT_INVALID`

#### 8.13 `ubl/sim.run`

Required fields:
- `episode_id`, `run_id`, `snapshot_cid`, `adapter_hash`, `runtime_hash`

Rules:
- `adapter_hash` must match allowlist from run request.

Example:
```json
{"@type":"ubl/sim.run","@id":"ep-001-sim-run-1","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","run_id":"run-0001","snapshot_cid":"b3:500","adapter_hash":"sha256:aaa","runtime_hash":"sha256:bbb"}
```

Deny:
- `DENY_RUNTIME_HASH_MISMATCH`

#### 8.14 `ubl/sim.result`

Required fields:
- `episode_id`, `run_id`, `result_cid`, `adapter_hash`, `runtime_hash`, `fuel_used`, `latency_ms`, `kpis`

Rules:
- `kpis` object must include `score`, `cost`, `integrity`.
- `fuel_used <= budget.fuel_cap`.

Example:
```json
{"@type":"ubl/sim.result","@id":"ep-001-sim-res-1","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","run_id":"run-0001","result_cid":"b3:600","adapter_hash":"sha256:aaa","runtime_hash":"sha256:bbb","fuel_used":120000,"latency_ms":80,"kpis":{"score":1.03,"cost":1.00,"integrity":0.99}}
```

Deny:
- `DENY_FUEL_CAP_EXCEEDED`
- `DENY_KPI_SCHEMA_INVALID`

#### 8.15 `ubl/run.link`

Required fields:
- `episode_id`, `run_id`, `request_receipt_cid`, `result_receipt_cid`

Rules:
- Both receipt CIDs must exist.

Example:
```json
{"@type":"ubl/run.link","@id":"ep-001-link-1","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","run_id":"run-0001","request_receipt_cid":"b3:701","result_receipt_cid":"b3:702"}
```

Deny:
- `DENY_RUN_LINK_MISSING_RECEIPT`

#### 8.16 Platform events (`web`, `mobile`, `cli`)

Required fields:
- `episode_id`, `platform_did`, `event_time`, `event_kind`, `payload`, `signature`, `kid`

Rules:
- Signature over canonical body must verify.
- Replay protection by nonce/sequence.

Example:
```json
{"@type":"ubl/platform.event.web","@id":"ep-001-web-1","@world":"a/lab/t/main","@ver":"1.0","episode_id":"001","platform_did":"did:ubl:web","event_time":"2026-02-23T00:10:00Z","event_kind":"session.heartbeat","payload":{"seq":1},"signature":"ed25519:...","kid":"did:ubl:web#ed25519"}
```

Deny:
- `DENY_PLATFORM_SIGNATURE_INVALID`
- `DENY_PLATFORM_REPLAY`

#### 8.17 `ubl/ai.passport`

Required fields:
- `issuer`, `subject`, `model_family`, `provider`, `capabilities`, `expiry`, `evidence`, `signature_did`

Rules:
- `expiry` must be future at validation time.
- `capabilities` non-empty.

Example:
```json
{"@type":"ubl/ai.passport","@id":"passport-gpt4o","@world":"a/lab/t/main","@ver":"1.0","issuer":"did:ubl:small","subject":"did:ubl:llm:gpt","model_family":"gpt","provider":"openai","capabilities":["propose","advise"],"expiry":"2026-12-31T23:59:59Z","evidence":{"attestation_cid":"b3:900"},"signature_did":"did:ubl:small#ed25519"}
```

Deny:
- `DENY_PASSPORT_CAPABILITY_EMPTY`
- `DENY_PASSPORT_SIGNATURE_INVALID`

### 9. Validation Placement in Runtime

- Small CHECK validates all control/method/advisory/publication types.
- Big CHECK validates `run.request`, execution/result types, platform events.
- Big rejects control-plane authority chips from non-small DIDs.

## 10. Files to Create or Change

Workspace and services:

- `/Users/ubl-ops/UBL-CORE/Cargo.toml`
- `/Users/ubl-ops/UBL-CORE/services/ubl_small_gate/Cargo.toml`
- `/Users/ubl-ops/UBL-CORE/services/ubl_small_gate/src/main.rs`
- `/Users/ubl-ops/UBL-CORE/services/ubl_big_gate/Cargo.toml`
- `/Users/ubl-ops/UBL-CORE/services/ubl_big_gate/src/main.rs`

Schema and validation:

- `/Users/ubl-ops/UBL-CORE/crates/ubl_schemas/Cargo.toml`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_schemas/src/lib.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_schemas/src/episode1/*.rs`
- `/Users/ubl-ops/UBL-CORE/schemas/episode1/*.schema.json`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_runtime/src/pipeline/check/episode1_small.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_runtime/src/pipeline/check/episode1_big.rs`

## 11. Implementation Checklist

1. Add `ubl_small_gate` and `ubl_big_gate` binaries wired to shared runtime.
2. Introduce feature flags and remove UI/MCP surfaces from Big.
3. Add `ubl_schemas` crate and canonical Episode 1 schema files.
4. Wire fail-closed CHECK validators by role.
5. Add integration tests for each mandatory type.
6. Add profile-specific CI jobs (`small`, `big`) with schema gates.
