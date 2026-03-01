# Episode 1 Protocol (Small/Big)

**Status**: active  
**Date**: 2026-02-22  
**Scope**: Televisionable + scientific test protocol for UBL

## Purpose

Episode 1 is a reproducible, auditable experiment that proves:

1. deterministic execution for core decisions,
2. LLM advisory without binding authority inside deterministic execution,
3. board-ready evidence package (receipts + provenance + video hash).

## Core Thesis

- Every demo is a test.
- Every test is a receipt trail.
- Recording is public proof, but publish is fail-closed.

## Planes and Roles

### `ubl-0` (Small, control plane)

Responsibilities:

1. governance engine (judge),
2. episode state machine (method lifecycle),
3. publish/archive decision,
4. evidence assembly and final bundle,
5. TV/OBS orchestration.

Non-responsibilities:

1. heavy simulation/materialization execution.

### `UBL-0` (Big, data plane)

Responsibilities:

1. deterministic heavy execution,
2. multi-platform ingest,
3. result production with receipts/evidence.

Non-responsibilities:

1. method governance decisions,
2. editorial/TV layer.

## Episode State Machine

1. `PREFLIGHT`
- verify storage, policies, attestations, recording readiness.

2. `DRAFT`
- method open for bounded edits.

3. `SEALED`
- method immutable (`protocol.seal`).

4. `RUNNING`
- Big executes, Small judges.

5. `PUBLISHED` or `ARCHIVED`
- fail-closed final state.

## Mandatory Artifacts for Publish

1. Verifiable receipts chain.
2. Complete episode bundle (provenance + indexes).
3. Video file hash sealed as `ubl/episode.video`.

If any of the three is missing, result is `ARCHIVED (UNPUBLISHED)`.

## Minimal Chip Taxonomy

Episode control:

1. `ubl/episode.start`
2. `ubl/episode.publish`
3. `ubl/episode.bundle`
4. `ubl/episode.video`

Method/control:

1. `ubl/dataset.spec.proposal`
2. `ubl/dataset.spec`
3. `ubl/protocol.seal`
4. `ubl/run.request`
5. `ubl/advisory`
6. `ubl/advisory.bundle`

Execution/result:

1. `ubl/dataset.materialize`
2. `ubl/dataset.snapshot`
3. `ubl/sim.run`
4. `ubl/sim.result`
5. `ubl/run.link`

Platform ingest (recommended):

1. `ubl/platform.event.web`
2. `ubl/platform.event.mobile`
3. `ubl/platform.event.cli`

## Determinism and Supply Chain

Execution profile: `deterministic_v1` with:

1. bounded fuel,
2. reproducible run inputs,
3. deny-by-policy on non-allowed capabilities,
4. fail-closed adapter/runtime attestation policy.

Current implementation note:

1. WASM execution already runs inside TR in runtime.
2. Full native LLM committee/quorum remains a control-plane evolution item.

## Governance and KPIs

Primary KPIs:

1. `Score`
2. `Cost`
3. `Integrity`

Audit KPIs:

1. `Replay Rate`
2. `Provenance Completeness`
3. `Fuel Burn p95`

Default publish thresholds (can be tightened):

1. replay divergence not accepted for publish,
2. provenance completeness >= 99%,
3. integrity above policy threshold.

## Episode Template

Use this block for each episode:

1. `hypothesis`
2. `goal`
3. `small_runtime_hash`
4. `big_runtime_hash`
5. `passports`
6. `method_spec_cid`
7. `policy_cid`
8. `execution_profile`
9. `adapter_hashes`
10. `kpis`
11. `decision` (`PUBLISHED` or `ARCHIVED`)
12. `bundle_cid` and `video_sha256`

## Acceptance Checklist (12 lines)

1. preflight passed
2. episode start receipt exists
3. method sealed receipt exists
4. run request receipt exists
5. big execution receipts exist
6. replay verification result recorded
7. provenance completeness measured
8. KPI thresholds evaluated
9. video recorded and hashed
10. `ubl/episode.video` emitted
11. bundle finalized and linked
12. final decision receipt emitted (`publish` or `archive`)

## Decision Rule

No video or incomplete provenance means archive.  
No exceptions.
