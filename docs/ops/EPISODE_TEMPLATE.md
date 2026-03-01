# Episode Template

## Header

1. `episode_id`:
2. `date_utc`:
3. `hypothesis`:
4. `goal`:

## Runtime Identity

1. `small_runtime_hash`:
2. `big_runtime_hash`:
3. `small_did`:
4. `big_did`:

## Method (Control Plane)

1. `method_spec_cid`:
2. `policy_cid`:
3. `protocol_seal_receipt_cid`:
4. `llm_passports`:

## Execution (Data Plane)

1. `execution_profile`:
2. `adapter_hashes`:
3. `run_request_receipt_cid`:
4. `result_receipt_cids`:

## Evidence

1. `bundle_cid`:
2. `video_sha256`:
3. `ledgers`:
4. `lineage_export`:

## KPIs

1. `score`:
2. `cost`:
3. `integrity`:
4. `replay_rate`:
5. `provenance_completeness`:
6. `fuel_burn_p95`:

## Decision

1. `decision`: `PUBLISHED` or `ARCHIVED`
2. `decision_reason`:
3. `final_receipt_cid`:
