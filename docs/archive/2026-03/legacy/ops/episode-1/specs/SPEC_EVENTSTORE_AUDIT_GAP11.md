# SPEC_EVENTSTORE_AUDIT_GAP11

Status: materialized
Date: 2026-02-23
Source prompt: 06
Scope: optimize `ubl_eventstore` audit queries for TV, export, and verifier workloads.

## 1. Event Model for Storage

### 1.1 Required Stored Fields

Normalized event must carry:

- `event_id` (stable unique id)
- `ts_ms` (monotonic ms for ordering)
- `@type`
- `@world`
- `stage`
- `actor_did`
- `decision`
- `code`
- optional refs: `chip_cid`, `receipt_cid`, `run_id`, `episode_id`

Current baseline:
- normalization already inserts `@id` and parses `when/timestamp`.
- indexes exist for world/stage/type/decision/code/actor/time.

Gap:
- no first-class run/episode indexes.
- no explicit canonicalization of enums.

### 1.2 Canonicalization Rules

Before insert (fail-closed):

- `stage`, `decision`, `code` normalized to uppercase canonical strings.
- `@world` and `@type` trimmed and validated.
- invalid timestamp rejects insert.

Reject codes:
- `EVENT_INVALID_TIMESTAMP`
- `EVENT_INVALID_WORLD`
- `EVENT_INVALID_TYPE`

## 2. Secondary Indexes and Keys

### 2.1 Index Trees

Keep existing trees and add two optional high-value indexes:

- `IDX_TIME` (`idx_time`)
- `IDX_TYPE` (`idx_type`)
- `IDX_WORLD` (`idx_world`)
- `IDX_STAGE` (`idx_stage`)
- `IDX_ACTOR` (`idx_actor`)
- `IDX_DECISION` (`idx_decision`)
- `IDX_CODE` (`idx_code`)
- `IDX_EPISODE` (`idx_episode`) new
- `IDX_RUN` (`idx_run`) new

### 2.2 Key Format

Use prefix-friendly format:

- time: `{:020}\x1f{event_id}`
- dimension: `{value}\x1f{:020}\x1f{event_id}`

Current code already uses this shape; keep unchanged for compatibility.

### 2.3 Payload Storage

- `events` tree stores full JSON by `event_id`.
- index trees store empty value and pointer in key suffix.

## 3. Query API Contract

### 3.1 Query Struct Target

Extend `EventQuery` with:

- `episode_id: Option<String>`
- `run_id: Option<String>`
- `order: Option<asc|desc>`
- `cursor: Option<String>`
- `end: Option<String>`

Keep existing:
- `world, stage, decision, code, chip_type, actor, since, limit`.

### 3.2 Endpoint Mapping

Primary endpoints using query engine:

- `GET /v1/events/search`
- `GET /v1/events` (stream historical seed)
- future:
  - `GET /v1/episodes/{id}/timeline`
  - `GET /v1/runs/{run_id}/timeline`
  - `GET /v1/exports/ledger.ndjson`

## 4. Query Planner

### 4.1 Planner Selection Order

Use most selective index in this order:

1. `run_id`
2. `episode_id`
3. exact `chip_type`
4. `actor`
5. `stage`
6. `decision`
7. `code`
8. `world`
9. fallback `IDX_TIME`

Current code already picks a best index; this spec formalizes and extends it.

### 4.2 Multi-Filter Strategy

- use one primary index scan as candidate generator.
- apply all remaining filters deterministically in memory.
- enforce range bounds (`since`, `end`) before pushing to page buffer.

### 4.3 Cursor and Stable Pagination

Cursor payload fields:

- `tree`
- `value`
- `last_ts_ms`
- `last_event_id`

Encoding:
- base64url JSON cursor.

Rules:
- no duplicates between pages.
- no skipped entries within same filter set.

### 4.4 Ordering

- default `desc` for audit search.
- `asc` allowed for playback/replayer.

Implementation:
- `asc`: forward range scan.
- `desc`: reverse scan or post-sort bounded candidate set.

## 5. Complexity Targets

- time-scan fallback: O(n) only when no selective filters.
- indexed scans: O(k) where k is candidate set for index value.
- page fetch should remain bounded by `limit` + modest overfetch.

## 6. Insert Path and Reindexing

### 6.1 Insert Path

On append:

1. validate + normalize event.
2. write event payload.
3. write `idx_time` entry.
4. write each applicable dimension index entry.
5. flush batch.

### 6.2 Reindex Command

Add reindex tooling:

- `ublx eventstore reindex --path <events_db>`

Behavior:
- clear index trees only.
- scan payload tree.
- rebuild all index trees including new `run` and `episode`.

Current `rebuild_indexes()` exists; CLI wrapper is missing.

## 7. Test Plan (Required)

### 7.1 Correctness

- insert mixed 1k events.
- verify filter by each index dimension.
- verify `run_id` and `episode_id` timeline queries.

### 7.2 Equivalence Property

For random queries:
- result(index planner) == result(full time scan + filter).

### 7.3 Cursor Stability

- paginate through full set.
- assert no duplicates and no omissions.

### 7.4 Performance

Benchmarks:
- compare `world/type/actor/run_id` indexed queries vs fallback scan.
- assert indexed path significantly lower scanned records.

### 7.5 Compatibility

- `judge.*` and `openlineage.*` appear in expected timeline order.
- existing `/v1/events/search` clients continue to work without cursor.

## 8. Acceptance Criteria

Accepted when:

- planner is explicit and deterministic.
- `/v1/events/search` supports cursor+order without breaking current params.
- new run/episode timelines are queryable efficiently.
- NDJSON export supports filters at query layer.
- reindex command exists and passes integration tests.

## Files to Change

- `/Users/ubl-ops/UBL-CORE/crates/ubl_eventstore/src/lib.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_cli/src/main.rs` (or eventstore subcommand module)
- `/Users/ubl-ops/UBL-CORE/services/ubl_gate/src/main.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_eventstore/tests/*`

## Implementation Order

1. Extend `EventQuery` and parser compatibility.
2. Add `IDX_RUN` and `IDX_EPISODE` plus extraction helpers.
3. Implement cursor+order in planner and search path.
4. Wire `/v1/events/search` cursor response and parsing.
5. Add CLI reindex command wrapper.
6. Add property, pagination, and perf tests.
