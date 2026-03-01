# SPEC_VERIFIER_EP1

Status: materialized
Date: 2026-02-23
Source prompt: 05
Scope: independent verifier binary for Episode 1 integrity, provenance completeness, determinism replay checks, and video linkage.

## 1. Verifier Interface (CLI)

### 1.1 Command

Primary command:

```bash
ubl-verify episode ./data/episodes/001/
```

### 1.2 Flags

- `--strict` (default true)
- `--replay-sample 0.05` (default 5%)
- `--export-md report.md`
- `--export-json report.json`
- `--big-url http://127.0.0.1:5000` (optional fallback source)

### 1.3 Mandatory Inputs

From episode directory:
- `bundle_001.json`
- `ledger_small.ndjson`
- `ledger_big.ndjson`
- `lineage_big.ndjson`
- `episodio_001.mp4` or `episodio_001.mkv`
- local CAS path (default `./data/cas`)

If any mandatory file is missing in strict mode:
- status `FAIL`
- exit code non-zero

## 2. Verification Model

### 2.1 File Integrity

Video check:
1. locate video entity in bundle
2. compute SHA-256 of referenced file
3. compare with bundle video hash and episode.video chip field

Failure code:
- `VIDEO_HASH_MISMATCH`

### 2.2 Bundle Integrity

Checks:
- JSON parse and schema validity
- required indexes exist (`run_to_receipts`, `type_to_cids`)
- each indexed CID appears in at least one ledger

Failure codes:
- `BUNDLE_SCHEMA_INVALID`
- `BUNDLE_INDEX_BROKEN`

### 2.3 Ledger Integrity

Checks:
- each line valid NDJSON
- monotonically non-decreasing timestamp
- required fields by kind (chip/receipt/event)

Optional chain check:
- if hash chain metadata exists, verify chain continuity.

Failure codes:
- `LEDGER_PARSE_ERROR`
- `LEDGER_ORDER_INVALID`
- `LEDGER_CHAIN_INVALID`

### 2.4 Receipt Integrity

For each referenced receipt:
- parse receipt JSON
- verify signature / auth chain (`current` and `prev` stage secrets)
- verify CID recomputation from canonical bytes

Failure codes:
- `RECEIPT_SIGNATURE_INVALID`
- `RECEIPT_CID_INVALID`

### 2.5 AI Passport Integrity

For each advisory/proposal in bundle:
- passport exists
- not expired at event time
- signature valid for passport issuer DID
- capability permits advisory action

Failure codes:
- `PASSPORT_MISSING`
- `PASSPORT_EXPIRED`
- `PASSPORT_SIGNATURE_INVALID`
- `PASSPORT_CAPABILITY_DENY`

### 2.6 OpenLineage Coherence

For each run in bundle index:
- has `START`
- has terminal `COMPLETE` or `FAIL`
- facets `ubl_*` required by contract are present
- facet CIDs/refs align with bundle and ledger

Failure codes:
- `LINEAGE_MISSING_START`
- `LINEAGE_MISSING_TERMINAL`
- `LINEAGE_FACET_MISSING`
- `LINEAGE_CID_MISMATCH`

### 2.7 PROV Coherence

Checks:
- required entities/activities/agents exist
- required relation edges exist
- no orphan run without receipt mapping

Failure codes:
- `PROV_ENTITY_MISSING`
- `PROV_RELATION_MISSING`
- `PROV_ORPHAN_RUN`

### 2.8 Determinism Replay

Default:
- sample 5% of runs from `run_to_receipts` index.

Strategy A (preferred):
- local deterministic re-execution with same runtime profile and CAS inputs.

Strategy B (fallback):
- verify replay witness receipt produced by trusted Small if local runtime unavailable.

Comparison set:
- `result_cid`
- `receipt_cid`
- `fuel_used`
- KPI tuple (`score`, `cost`, `integrity`)

Failure codes:
- `REPLAY_DIVERGENCE`
- `REPLAY_INPUT_MISSING`
- `REPLAY_RUNTIME_UNAVAILABLE` (strict => fail; non-strict => warn)

## 3. Output Contracts

### 3.1 `report.md` (board-ready)

One-page summary sections:
- final status: `PUBLISHED` or `ARCHIVED` or `FAIL`
- reason and highest severity failures
- KPI summary (Score/Cost/Integrity)
- audit KPIs (Replay Rate, Provenance Completeness, Fuel Burn p95)
- video proof (`sha256`, file path)
- lineage completeness metrics

### 3.2 `report.json` (machine)

Shape:

```json
{
  "status": "FAIL",
  "episode_id": "001",
  "checks": [
    {"id":"video_hash","pass":false,"code":"VIDEO_HASH_MISMATCH","detail":"..."}
  ],
  "kpis": {
    "score": 1.01,
    "cost": 1.00,
    "integrity": 0.98,
    "replay_rate": 0.95,
    "provenance_completeness": 0.97,
    "fuel_burn_p95": 300000
  },
  "cids": {
    "bundle_cid": "b3:...",
    "video_chip_cid": "b3:..."
  },
  "divergences": [
    {"run_id":"run-0021","reason":"result_cid mismatch"}
  ]
}
```

Determinism rule:
- identical inputs produce byte-identical `report.json` in strict mode.

## 4. Exit and Decision Rules

Exit codes:
- `0`: all strict checks pass
- `2`: archived by policy but verifier checks passed for archive path
- `1`: verifier failure

Decision precedence:
1. hard integrity failures => `FAIL`
2. completeness/replay threshold breaches => `FAIL` in strict mode
3. otherwise emit `PUBLISHED`/`ARCHIVED` according to bundle decision

## 5. Acceptance Criteria

Must detect and fail on:
- tampered video file
- advisory without valid passport
- run without lineage start/terminal
- replay divergence in sampled runs

Must run:
- locally without internet
- using only episode folder + local CAS by default

Must generate:
- `report.md`
- `report.json`

## 6. Rust Module Structure

New crate:
- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/`

Modules:
- `io`
- `cas`
- `ledger`
- `receipts`
- `passports`
- `lineage`
- `prov`
- `replay`
- `report`

Light dependencies:
- `serde`, `serde_json`, `sha2`, `blake3`, `clap`, optional `reqwest` for `--big-url`

## 7. Implementation Order

1. Create verifier crate and CLI command skeleton.
2. Implement file loading and strict input validation.
3. Implement video hash and bundle/ledger checks.
4. Implement receipt and passport verification.
5. Implement lineage/prov consistency checks.
6. Implement replay sampling path.
7. Emit deterministic markdown/json reports.
8. Integrate with episode runner finalization gate.

## Files to Create or Change

- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/Cargo.toml`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/src/main.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/src/lib.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/src/io.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/src/ledger.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/src/receipts.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/src/passports.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/src/lineage.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/src/prov.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/src/replay.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_verifier/src/report.rs`
- `/Users/ubl-ops/UBL-CORE/Cargo.toml` (workspace member)
- `/Users/ubl-ops/UBL-CORE/docs/ops/episode-1/specs/SPEC_EPISODE_RUNNER_EP1.md` (runner hook reference)
