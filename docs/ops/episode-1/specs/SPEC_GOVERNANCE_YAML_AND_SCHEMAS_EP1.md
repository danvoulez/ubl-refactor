# SPEC_GOVERNANCE_YAML_AND_SCHEMAS_EP1

Status: materialized
Date: 2026-02-23
Source prompt: 08
Scope: Episode 1 governance config, fail-closed YAML validator, and strict chip contracts.

## 1. `lab.governance.v0.yaml` Structure

### 1.1 Design Decision

- One authoritative YAML file controls governance defaults for both planes.
- Small loads the full file.
- Big loads a strict subset (`determinism`, `supply_chain`, `lineage`, `identity`).
- Any invalid/missing mandatory field aborts boot.

### 1.2 Canonical Top-Level Shape

```yaml
apiVersion: lab.governance/v0
metadata:
  name: episode-1-governance
  revision: 1
  generated_at: "2026-02-23T00:00:00Z"
episode:
  id_prefix: "ep"
  max_draft_minutes: 30
  autoseal_idle_minutes: 10
  preflight:
    obs_required: true
    attest_required: true
    storage_required: true
    verifier_required: true
identities:
  dids:
    small: "did:ubl:small"
    big: "did:ubl:big"
    platforms:
      web: "did:ubl:web"
      mobile: "did:ubl:mobile"
      cli: "did:ubl:cli"
committee:
  passports:
    required: true
  quorum:
    required_approvals: 2
    pool_size: 3
    diversity_key: "model_family"
    safety_veto:
      enabled: true
      did: "did:ubl:llm:safety"
  allowed_models:
    - family: "gpt"
      provider: "openai"
      min_version: "4o"
      passport_issuer_allowlist:
        - "did:ubl:small"
    - family: "claude"
      provider: "anthropic"
      min_version: "sonnet-4"
      passport_issuer_allowlist:
        - "did:ubl:small"
determinism:
  execution_profile: "deterministic_v1"
  wasm:
    virtual_clock: true
    rng_seed_source: "spec.seed"
    forbid_threads: true
    forbid_network: true
    env_allowlist:
      - "UBL_PROFILE"
    fs_allowlist:
      - "./data/cas/readonly"
  budgets:
    fuel_cap_per_run: 500000
    max_latency_ms_p95: 1500
supply_chain:
  attestations:
    required: true
    verifier: "cosign"
    attest_dir: "./security/attestations"
    fail_closed: true
lineage:
  openlineage:
    required: true
    emit_start: true
    emit_complete: true
    facets_schema_url_base: "https://schemas.ubl.agency/openlineage/ubl/v1"
  prov:
    required: true
    completeness_threshold: 0.99
kpis:
  score:
    promote_delta_min: 0.02
  cost:
    promote_delta_max: 0.01
  integrity:
    floor: 0.95
  replay_rate:
    must_be: 1.0
  replay_sample_rate: 0.05
  fuel_burn_p95:
    cap: 500000
publish:
  require_video: true
  require_video_hash: true
  on_no_video: "ARCHIVE"
  on_low_completeness: "ARCHIVE"
  on_replay_divergence: "ARCHIVE"
  archive_reasons_enum:
    - "NO_VIDEO"
    - "VIDEO_HASH_MISMATCH"
    - "LOW_COMPLETENESS"
    - "REPLAY_DIVERGENCE"
    - "NO_QUORUM"
    - "PREFLIGHT_FAILED"
    - "VERIFY_FAIL"
judge_events:
  required:
    - "judge.passport.deny"
    - "judge.autoseal"
    - "judge.replay.divergence"
    - "judge.publish.archive"
observability:
  tv:
    enabled: true
    sse_path: "/tv/stream"
    big_is_larger_rule: true
  obs:
    websocket_url: "ws://127.0.0.1:4455"
    record_container: "mkv"
    remux_to: "mp4"
    output_dir: "./data/episodes/{{episode_id}}/"
identity_policy:
  world_pattern: "a/{app}/t/{tenant}"
  require_subject_did_on_sensitive_types: true
  public_ingest_world: "a/chip-registry/t/public"
```

### 1.3 Required Enums and Bounds

- `apiVersion`: must equal `lab.governance/v0`.
- `determinism.execution_profile`: only `deterministic_v1`.
- `supply_chain.attestations.verifier`: one of `cosign`, `dsse`.
- `publish.on_*`: must be `ARCHIVE`.
- `kpis.integrity.floor`: `0.0..=1.0`.
- `kpis.replay_rate.must_be`: must be `1.0`.
- `lineage.prov.completeness_threshold`: `0.0..=1.0`.
- `committee.quorum.required_approvals <= pool_size`.

## 2. YAML Validation (Rust, Fail-Closed)

### 2.1 Load Path and Boot Order

- Small boot sequence:
1. load file from `UBL_GOVERNANCE_YAML` (default `./ops/lab.governance.v0.yaml`)
2. parse YAML (`serde_yaml`)
3. semantic validation
4. initialize governance engine
5. only then bind episode mutation endpoints

- Big boot sequence:
1. load same file
2. validate subset (determinism, supply-chain, lineage)
3. initialize runtime enforcement profile
4. only then accept `run.request` and platform events

### 2.2 Fail-Closed Rules

Abort boot when any condition is true:

- file missing or unreadable
- parse failure
- unknown `apiVersion`
- unknown fields when strict mode is enabled
- invalid DID format in required identities
- inconsistent thresholds (for example replay target not `1.0`)
- `require_video=true` with missing OBS block
- `attest_required=true` with empty `attest_dir`

### 2.3 Rust Type Layout (Target)

- `crates/ubl_governance/src/config.rs`
  - `LabGovernanceV0`
  - `EpisodeCfg`
  - `IdentityCfg`
  - `CommitteeCfg`
  - `DeterminismCfg`
  - `SupplyChainCfg`
  - `LineageCfg`
  - `KpiCfg`
  - `PublishCfg`
  - `ObsCfg`

- `crates/ubl_governance/src/validate.rs`
  - `validate_governance(&LabGovernanceV0) -> Result<(), GovernanceError>`

### 2.4 Error Model

All config errors must map to deterministic codes:

- `GOV_CFG_FILE_NOT_FOUND`
- `GOV_CFG_PARSE_ERROR`
- `GOV_CFG_VERSION_UNSUPPORTED`
- `GOV_CFG_ENUM_INVALID`
- `GOV_CFG_RANGE_INVALID`
- `GOV_CFG_IDENTITY_INVALID`
- `GOV_CFG_POLICY_INCONSISTENT`

## 3. Schema Contracts for Episode 1 Types

### 3.1 Global Envelope Contract (all types)

Every chip must include:

- `@type`: string
- `@id`: string
- `@ver`: string
- `@world`: string matching `a/{app}/t/{tenant}` except system-reserved denials

Global deny codes:

- `INVALID_ENVELOPE`
- `INVALID_WORLD`
- `UNSUPPORTED_TYPE`
- `INVALID_VERSION`

### 3.2 Required Type Set

- `ubl/episode.start`
- `ubl/episode.publish`
- `ubl/episode.bundle`
- `ubl/episode.video`
- `ubl/dataset.spec.proposal`
- `ubl/dataset.spec`
- `ubl/protocol.seal`
- `ubl/run.request`
- `ubl/advisory`
- `ubl/advisory.bundle`
- `ubl/dataset.materialize`
- `ubl/dataset.snapshot`
- `ubl/sim.run`
- `ubl/sim.result`
- `ubl/run.link`
- `ubl/platform.event.web`
- `ubl/platform.event.mobile`
- `ubl/platform.event.cli`
- `ubl/ai.passport`

### 3.3 Per-Type Mandatory Fields (strict)

`ubl/episode.start`
- required: `episode_id`, `seed`, `policy_cid`
- deny on: missing `seed`, invalid `policy_cid`

`ubl/episode.publish`
- required: `episode_id`, `decision`, `reason`
- `decision` enum: `PUBLISHED`, `ARCHIVED`

`ubl/episode.bundle`
- required: `episode_id`, `bundle_cid`, `lineage_path`, `ledger_small_path`, `ledger_big_path`

`ubl/episode.video`
- required: `episode_id`, `sha256`, `container`, `path`, `started_at`, `ended_at`, `episode_bundle_cid`
- `container` enum: `mkv`, `mp4`

`ubl/dataset.spec.proposal`
- required: `episode_id`, `author_did`, `proposal_hash`, `content_hash`

`ubl/dataset.spec`
- required: `episode_id`, `spec_cid`, `seed`, `generator_version`

`ubl/protocol.seal`
- required: `episode_id`, `sealed_by_did`, `method_spec_cid`, `sealed_at`

`ubl/run.request`
- required: `episode_id`, `run_id`, `spec_cid`, `policy_cid`, `execution_profile`, `budget`, `allowed_adapter_hashes`
- `execution_profile` must equal `deterministic_v1`

`ubl/advisory`
- required: `episode_id`, `passport_cid`, `model_identity`, `prompt_hash`, `input_hash`, `advisory_hash`

`ubl/advisory.bundle`
- required: `episode_id`, `advisory_cids`, `quorum_summary`

`ubl/dataset.materialize`
- required: `episode_id`, `run_id`, `spec_cid`, `snapshot_target`

`ubl/dataset.snapshot`
- required: `episode_id`, `run_id`, `snapshot_cid`, `record_count`

`ubl/sim.run`
- required: `episode_id`, `run_id`, `snapshot_cid`, `adapter_hash`, `runtime_hash`

`ubl/sim.result`
- required: `episode_id`, `run_id`, `result_cid`, `adapter_hash`, `runtime_hash`, `fuel_used`, `latency_ms`, `kpis`

`ubl/run.link`
- required: `episode_id`, `run_id`, `request_receipt_cid`, `result_receipt_cid`

`ubl/platform.event.web|mobile|cli`
- required: `episode_id`, `platform_did`, `event_time`, `event_kind`, `payload`, `signature`, `kid`

`ubl/ai.passport`
- required: `issuer`, `subject`, `model_family`, `provider`, `capabilities`, `expiry`, `evidence`, `signature_did`

### 3.4 Schema Strategy Decision

Decision: hybrid model.

- Canonical JSON schema files in repo for contract transparency.
- Rust `serde` structs + manual semantic validators in CHECK for fail-closed behavior.

Paths:
- `schemas/episode1/*.schema.json`
- `crates/ubl_schemas/src/episode1/*.rs`
- `crates/ubl_runtime/src/pipeline/check/episode1_validate.rs`

## 4. Optional Codegen (Constrained)

Codegen may generate base structs only.
Custom validation remains handwritten and mandatory.

If enabled:
- use generated structs under `crates/ubl_schemas/src/generated/`
- convert into validated domain structs before runtime use

## 5. Full YAML Example for Commit

Use this file path:
- `/Users/ubl-ops/UBL-CORE/ops/lab.governance.v0.yaml`

```yaml
apiVersion: lab.governance/v0
metadata:
  name: episode-1-governance
  revision: 1
  generated_at: "2026-02-23T00:00:00Z"

episode:
  id_prefix: "ep"
  max_draft_minutes: 30
  autoseal_idle_minutes: 10
  preflight:
    obs_required: true
    attest_required: true
    storage_required: true
    verifier_required: true

identities:
  dids:
    small: "did:ubl:small"
    big: "did:ubl:big"
    platforms:
      web: "did:ubl:web"
      mobile: "did:ubl:mobile"
      cli: "did:ubl:cli"

committee:
  passports:
    required: true
  quorum:
    required_approvals: 2
    pool_size: 3
    diversity_key: "model_family"
    safety_veto:
      enabled: true
      did: "did:ubl:llm:safety"
  allowed_models:
    - family: "gpt"
      provider: "openai"
      min_version: "4o"
      passport_issuer_allowlist: ["did:ubl:small"]
    - family: "claude"
      provider: "anthropic"
      min_version: "sonnet-4"
      passport_issuer_allowlist: ["did:ubl:small"]

determinism:
  execution_profile: "deterministic_v1"
  wasm:
    virtual_clock: true
    rng_seed_source: "spec.seed"
    forbid_threads: true
    forbid_network: true
    env_allowlist: ["UBL_PROFILE"]
    fs_allowlist: ["./data/cas/readonly"]
  budgets:
    fuel_cap_per_run: 500000
    max_latency_ms_p95: 1500

supply_chain:
  attestations:
    required: true
    verifier: "cosign"
    attest_dir: "./security/attestations"
    fail_closed: true

lineage:
  openlineage:
    required: true
    emit_start: true
    emit_complete: true
    facets_schema_url_base: "https://schemas.ubl.agency/openlineage/ubl/v1"
  prov:
    required: true
    completeness_threshold: 0.99

kpis:
  score:
    promote_delta_min: 0.02
  cost:
    promote_delta_max: 0.01
  integrity:
    floor: 0.95
  replay_rate:
    must_be: 1.0
  replay_sample_rate: 0.05
  fuel_burn_p95:
    cap: 500000

publish:
  require_video: true
  require_video_hash: true
  on_no_video: "ARCHIVE"
  on_low_completeness: "ARCHIVE"
  on_replay_divergence: "ARCHIVE"
  archive_reasons_enum:
    - "NO_VIDEO"
    - "VIDEO_HASH_MISMATCH"
    - "LOW_COMPLETENESS"
    - "REPLAY_DIVERGENCE"
    - "NO_QUORUM"
    - "PREFLIGHT_FAILED"
    - "VERIFY_FAIL"

judge_events:
  required:
    - "judge.passport.deny"
    - "judge.autoseal"
    - "judge.replay.divergence"
    - "judge.publish.archive"

observability:
  tv:
    enabled: true
    sse_path: "/tv/stream"
    big_is_larger_rule: true
  obs:
    websocket_url: "ws://127.0.0.1:4455"
    record_container: "mkv"
    remux_to: "mp4"
    output_dir: "./data/episodes/{{episode_id}}/"

identity_policy:
  world_pattern: "a/{app}/t/{tenant}"
  require_subject_did_on_sensitive_types: true
  public_ingest_world: "a/chip-registry/t/public"
```

## 6. Files to Create or Change

- `/Users/ubl-ops/UBL-CORE/ops/lab.governance.v0.yaml`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_governance/Cargo.toml`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_governance/src/config.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_governance/src/validate.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_governance/src/lib.rs`
- `/Users/ubl-ops/UBL-CORE/schemas/episode1/*.schema.json`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_runtime/src/pipeline/check/episode1_validate.rs`
- `/Users/ubl-ops/UBL-CORE/services/ubl_gate/src/main.rs` (boot wiring)

## 7. Implementation Checklist

1. Add YAML file and governance crate with strict parse/validate.
2. Wire Small and Big boot loaders to governance config.
3. Add Episode 1 schemas in `schemas/episode1/`.
4. Add runtime CHECK validators and deterministic deny codes.
5. Add tests: valid YAML, invalid YAML, per-type deny coverage.
6. Gate episode endpoints until governance loads cleanly.
