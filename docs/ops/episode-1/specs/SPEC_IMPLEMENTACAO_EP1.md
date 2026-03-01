# SPEC_IMPLEMENTACAO_EP1

Status: materialized
Date: 2026-02-23
Source prompt: 01
Scope: integrated implementation contract for Episode 1 across Small, Big, and Infra-0.

## A. `ubl-0` (Small, Control Plane, Judge + Producer)

### A1. Responsibilities

Does:
- governance engine (policy + episode state machine + verifier gate)
- method lifecycle (`DRAFT -> SEALED`)
- publish/archive decision authority
- bundle assembly and final evidence publication
- TV/OBS orchestration

Does not:
- heavy dataset/simulation execution
- raw large-scale platform ingestion as primary workload

### A2. Internal Components

- `ubl_runtime` shared pipeline
- `ubl_governance` config + semantic validator
- `judge` module (state machine + decisions)
- `obs` module (preflight/start/stop/remux/hash)
- `bundle` module (PROV assembly)
- event/broadcast integration for TV

### A3. Endpoints (Small)

Control flow endpoints (implemented via chips; dedicated endpoints optional):
- `POST /v1/chips` (episode control chips)
- `GET /v1/events`
- `GET /v1/events/search`

Recommended explicit episode endpoints:
- `POST /v1/episode/start`
- `GET /v1/episode/status`
- `POST /v1/episode/publish`

Responses must include deterministic error codes and receipt references when available.

### A4. Chips and Receipts

Consumes/emits:
- `ubl/episode.start`
- `ubl/protocol.seal`
- `ubl/run.request`
- `ubl/episode.publish`
- `ubl/episode.bundle`
- `ubl/episode.video`
- `ubl/advisory*`

Receipts:
- all decisions are receipt-backed
- no side-channel state transitions

### A5. Main Flow

1. PREFLIGHT checks pass
2. start chip accepted
3. method proposals/advisories collected
4. autoseal or quorum seal
5. run.request emitted to Big
6. monitor KPIs and evidence
7. publish/archive decision
8. finalize bundle + video + verification

### A6. Persistence

Small persists:
- SQLite durable state
- EventStore events
- ledger NDJSON
- bundle JSON artifacts

Minimum indexes for judge queries:
- episode_id
- run_id
- decision
- code

### A7. Observability

Feeds TV from:
- EventStore query + stream
- judge event emissions

Required judge events:
- `judge.passport.deny`
- `judge.autoseal`
- `judge.replay.divergence`
- `judge.publish.archive`

### A8. Security and Policy

- fail-closed on missing governance config
- fail-closed on required passport/attestation gaps
- `no video -> no publish`
- quorum and diversity enforced before seal/publish

### A9. OBS Integration

- preflight verifies connectivity and output path
- record in MKV only
- remux to MP4 for publish
- hash final media and bind into `ubl/episode.video`

### A10. Acceptance for Small

Ready when:
- state transitions are deterministic and receipt-backed
- publish path archives on any mandatory artifact gap
- bundle includes required provenance links

---

## B. `UBL-0` (Big, Data Plane, Heavy Deterministic Execution)

### B1. Responsibilities

Does:
- process deterministic heavy runs
- ingest signed platform events
- execute WASM adapters under deterministic constraints
- emit lineage and receipts

Does not:
- governance decisions
- editorial/UI concerns

### B2. Internal Components

- `ubl_runtime` TR execution path
- WASM adapter executor (`wasmtime`) with fuel/memory guard
- attestation verification gate
- OpenLineage emitter
- EventStore write path

### B3. Endpoints (Big)

- `GET /healthz`
- `POST /v1/chips`
- `GET /v1/chips/:cid`
- `GET /v1/receipts/:cid`
- `GET /v1/events/search`

No UI/MCP endpoints in Big profile.

### B4. Chips and Receipts

Primary types:
- `ubl/platform.event.*`
- `ubl/dataset.materialize`
- `ubl/dataset.snapshot`
- `ubl/sim.run`
- `ubl/sim.result`
- `ubl/run.link`

All execution outputs produce receipts and lineage records.

### B5. Flow

1. validate signed run request origin and policy
2. accept platform events under per-DID limits
3. execute deterministic TR path
4. emit `START` and terminal lineage events
5. persist execution receipts and references

### B6. Persistence

- CAS for content artifacts
- SQLite durable state
- EventStore with indexed query paths
- per-episode lineage NDJSON export

### B7. Observability

- publish operational events to EventStore
- expose metrics for latency, fuel, denial codes

### B8. Security and Determinism

- execution profile must be `deterministic_v1`
- no network, no threads, no nondeterministic clock in WASM adapters
- adapter/runtime attestation required in strict mode

### B9. Acceptance for Big

Ready when:
- deterministic run replay consistency is proven in verifier sample
- lineage terminal coverage is complete for all indexed runs
- rejection codes are explicit and queryable

---

## C. `Codigo-Infra-0` (Local Studio)

### C1. Responsibilities

- local composition of Small + Big + TV + mocks + verifier
- one-command orchestration
- artifact directory control

### C2. Processes

- small
- big
- tv
- web/mobile/cli mocks
- verifier

### C3. Local Contracts

- no cloud requirement
- deterministic seed-driven execution
- all outputs under `./data/episodes/{id}/`

### C4. File/Artifact Set

Mandatory episode artifacts:
- `bundle_{id}.json`
- `ledger_small.ndjson`
- `ledger_big.ndjson`
- `lineage_big.ndjson`
- `episodio_{id}.mkv`
- `episodio_{id}.mp4` (if remux)
- `report.md`
- `report.json`

### C5. Acceptance for Infra-0

Ready when:
- `just episode-1` completes end-to-end without manual edits
- verifier can run offline on produced artifact set

---

## Cross-Cutting Contracts

### Governance Engine

Judge combines:
- policy checks
- state machine enforcement
- verifier gating for final outcome

### PREFLIGHT Required Checks

- OBS ready
- storage writable
- attestations available
- allowlist loaded
- governance YAML loaded
- verifier available

If any fail -> archive/no start.

### Multi-Platform Ingest vs Run Authority

- platforms may send signed `ubl/platform.event.*` directly to Big
- only Small authority DID may issue canonical `ubl/run.request`

### OpenLineage and PROV Link

- Big emits OpenLineage START/terminal events with `ubl_*` facets
- Small assembles PROV bundle and links lineage/ledgers/video hashes

### Deterministic WASM Profile

- virtual clock
- seeded RNG
- no threads
- no real network
- bounded memory and fuel

Failure codes:
- `FAIL_FUEL_CAP`
- `DENY_NONDETERMINISTIC`

### Attestation Fail-Closed

- verify runtime/adapter attestations before execution
- attestation failure blocks run execution

### KPI Threshold Defaults

- score delta >= +2%
- cost delta <= +1%
- integrity >= 95%
- replay rate = 100%
- provenance completeness >= 99%

### Publish Rule

- missing video or missing hash always archives

---

## 2-Week Implementation Checklist (Order Only)

1. Governance YAML loader + strict validator.
2. Split service profiles Small/Big and route gating.
3. GAP-6/GAP-15 persistence hardening.
4. EventStore GAP-11 index planner and pagination.
5. OpenLineage emitter and NDJSON export.
6. PROV bundle assembler and completeness index.
7. OBS automation with fail-closed publish integration.
8. Platform mocks deterministic generators.
9. Independent verifier CLI.
10. Episode runner one-command orchestration.
11. Full LAB 256 rehearsal and evidence pack.
12. Promotion checklist signoff for LAB 512.

## Suggested Repository Targets

- `/Users/ubl-ops/UBL-CORE/services/ubl_small_gate/`
- `/Users/ubl-ops/UBL-CORE/services/ubl_big_gate/`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_governance/`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_schemas/`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_lineage/`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_platform_mocks/`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_episode_runner/`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/`
- `/Users/ubl-ops/UBL-CORE/ops/lab.governance.v0.yaml`
- `/Users/ubl-ops/UBL-CORE/docs/ops/episode-1/specs/`
