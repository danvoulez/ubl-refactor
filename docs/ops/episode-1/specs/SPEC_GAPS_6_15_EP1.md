# SPEC_GAPS_6_15_EP1

Status: materialized
Date: 2026-02-23
Source prompt: 07
Scope: harden GAP-6 (nonce replay) and GAP-15 (stage secret rotation) for fail-closed Episode 1 operations.

## Section A - GAP-6 seen_nonces Persistent with TTL

### A1. Threat and Requirements

Threat addressed:
- replay across process restart for the same authorship scope and nonce.

Not addressed:
- unique new nonce submissions; those are handled by idempotency and policy.

Requirements:
- persistent nonce checks in SQLite.
- TTL default 24h (configurable).
- strict mode fail-closed: storage failure denies write.
- deterministic denial code for replay.

Current code baseline:
- table `seen_nonces` exists.
- method `nonce_mark_if_new(nonce, ttl)` exists.
- nonce currently global key only (no scope dimension).

### A2. SQLite Schema Target

Current schema:
- `seen_nonces(nonce PRIMARY KEY, created_at, expires_at)`.

Target schema:

```sql
CREATE TABLE IF NOT EXISTS seen_nonces (
  scope         TEXT NOT NULL,
  nonce         TEXT NOT NULL,
  first_seen_ts INTEGER NOT NULL,
  expires_ts    INTEGER NOT NULL,
  meta_json     TEXT,
  PRIMARY KEY (scope, nonce)
);
CREATE INDEX IF NOT EXISTS idx_seen_nonces_expires ON seen_nonces (expires_ts);
CREATE INDEX IF NOT EXISTS idx_seen_nonces_scope_expires ON seen_nonces (scope, expires_ts);
```

Scope format:
- default: `{world}|{subject_did}`.
- fallback (no subject): `{world}|anon`.

### A3. DurableStore API Target

Add:

- `check_and_insert_nonce(scope, nonce, now_ms, ttl_ms) -> Result<NonceStatus>`
  - returns `NonceStatus::New` or `NonceStatus::Replay`.
  - implementation in one transaction.

- `prune_expired_nonces(now_ms, max_delete) -> Result<u64>`
  - bounded prune for predictable write path.

Suggested enum:

```rust
pub enum NonceStatus {
    New,
    Replay,
}
```

Migration approach:
- keep existing `nonce_mark_if_new` as compatibility wrapper for one cycle.
- wrapper maps to scoped API with `scope="legacy"`.

### A4. Pipeline Integration (Exact Point)

Integration point:
- `crates/ubl_runtime/src/pipeline/processing.rs` in WA pre-accept path.

Execution order:

1. resolve `subject_did` and `world`.
2. derive `scope = format!("{}|{}", world, subject_did_or_anon)`.
3. run in-memory fast check.
4. run durable check via `check_and_insert_nonce`.
5. if replay -> `PipelineError::ReplayDetected("replay: nonce already seen")`.

Error mapping:
- replay => `DENY_REPLAY` (HTTP 409).
- durable failure in strict mode => `STORAGE_UNAVAILABLE` (deny).
- durable failure in non-strict mode => warn + in-memory only.

Config keys:
- `NONCE_TTL_MS` default `86400000`.
- `NONCE_STRICT` default `true` for prod, `false` for dev.
- `NONCE_PRUNE_BATCH` default `500`.

### A5. Migration and Compatibility

Migration strategy:

1. create new scoped table and indexes.
2. read old table if present and import rows to `scope='legacy'` with same expiry.
3. stop writing old table.
4. optional cleanup migration in later release.

Boot behavior:
- migration runs in `DurableStore::ensure_initialized()`.
- failure to migrate in strict mode aborts startup.

### A6. Tests (GAP-6)

Unit:
- new then replay for same `(scope, nonce)`.
- same nonce different scope is `New`.
- expiry restores `New`.
- prune respects batch limit.

Integration:
- restart preserves replay detection.
- strict mode denies when sqlite is read-only.
- non-strict mode warns and continues with in-memory only.

---

## Section B - GAP-15 Stage Secret Rotation Chain

### B1. Requirements

After `ubl/key.rotate` succeeds:

1. move current stage secret to previous.
2. derive new stage secret from new signing key.
3. persist (`current`, `prev`, `rotated_at`, `rotation_cid`) durably.
4. apply runtime values atomically.
5. survive restart.

Current baseline:
- `derive_stage_secret` exists (keyed BLAKE3).
- `put_stage_secrets(current, prev)` exists.
- `get_stage_secrets()` and boot-time env apply exist.
- `rotation_cid` is not persisted yet.
- env set currently occurs before persistence call.

### B2. Derivation Contract

Decision (explicit):
- keep deterministic derivation from signing key:

`stage_secret = blake3_keyed(signing_key_bytes, "ubl.stage_secret.v1")`

Encoding:
- env/storage string format: `hex:<64-hex>`.

Rationale:
- stable with existing runtime behavior.
- no dependency on clock or external entropy.

### B3. SQLite Schema Target

Current table:
- `stage_secrets(id=1, current, prev, rotated_at)`.

Target:

```sql
CREATE TABLE IF NOT EXISTS stage_secrets (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  current_secret TEXT NOT NULL,
  prev_secret    TEXT,
  rotated_at_ts  INTEGER NOT NULL,
  rotation_cid   TEXT,
  meta_json      TEXT
);
```

Compatibility migration:
- if old columns exist, migrate values into new columns.
- provide view or code fallback during transition.

### B4. DurableStore API Target

Add:
- `load_stage_secrets() -> Result<Option<StageSecrets>, DurableError>`
- `store_stage_secrets(current, prev, rotated_at, rotation_cid) -> Result<()>`

Type:

```rust
pub struct StageSecrets {
    pub current: String,
    pub prev: Option<String>,
    pub rotated_at_ts: i64,
    pub rotation_cid: Option<String>,
}
```

### B5. Boot Integration Order

Required order:

1. load signing key.
2. initialize durable store.
3. read persisted stage secrets.
4. if present, override env values from persisted row.
5. if absent and `STAGE_SECRET_PERSIST_ON_BOOT=true`, persist derived initial current.

Current behavior already applies step 3-4; keep and extend with strict errors.

Strict mode:
- `STAGE_SECRET_STRICT=true` in prod.
- if persisted row exists but invalid/unreadable, abort boot.

### B6. Rotation Flow (Atomicity)

Current flow sets env before persist. Target flow must be transactional in effect:

1. derive new signing key and new stage secret.
2. snapshot previous stage secret.
3. persist row first (`store_stage_secrets`).
4. only after success, set `UBL_STAGE_SECRET` and `UBL_STAGE_SECRET_PREV`.
5. emit judge event `judge.key.rotate` with `rotation_cid`.

Failure handling:
- if persist fails, do not mutate env runtime secrets.
- return pipeline storage error and deny rotation completion.

### B7. Tests (GAP-15)

Unit:
- derivation deterministic golden.
- load/store roundtrip includes `rotation_cid`.

Integration:
- rotate then restart preserves chain and verification accepts current+prev.
- read-only DB causes rotation failure and leaves env unchanged.
- migration from old schema retains values.

### B8. Configuration Keys

- `STAGE_SECRET_STRICT` default true in prod.
- `STAGE_SECRET_PERSIST_ON_BOOT` default true.

---

## Section C - Acceptance Criteria

GAP-6 accepted when:
- replay is detected across restart by scoped nonce key.
- TTL and prune work under load.
- WA denial uses explicit replay code.

GAP-15 accepted when:
- rotation updates current/prev and persists atomically.
- restart restores same secret chain from DB.
- persistence failure aborts rotation and avoids partial runtime mutation.

## Files to Change

- `/Users/ubl-ops/UBL-CORE/crates/ubl_runtime/src/durable_store.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_runtime/src/pipeline/processing.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_runtime/src/pipeline/mod.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_runtime/src/error_response.rs`
- `/Users/ubl-ops/UBL-CORE/crates/ubl_runtime/src/pipeline/tests.rs`

## Implementation Order

1. Add DB migrations for scoped nonces and stage secret metadata columns.
2. Add new DurableStore APIs and compatibility wrappers.
3. Move WA nonce enforcement to scoped API.
4. Refactor key rotation flow to persist-before-env mutation.
5. Add strict-mode config handling.
6. Add unit/integration tests and restart scenarios.
