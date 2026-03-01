# Episode 1 Prompt Pack Intake

Status: active intake
Date: 2026-02-23
Source pack: `/Users/ubl-ops/ExperimentoUBL/prompts/01..09`

## Goal

Capture the 9 prompt documents as an executable engineering backlog for Episode 1, without losing scope or introducing parallel conflicting specs.

## Prompt Inventory

1. `01_prompt_spec_implementacao.txt`
- Output target: `SPEC_IMPLEMENTACAO_EP1.md`
- Scope: full A/B/C architecture (Small, Big, Infra-0), contracts, flows, persistence, security, OBS, acceptance.
- Role in plan: umbrella spec.

2. `02_prompt_workspace_schemas.txt`
- Output target: `SPEC_WORKSPACE_E_SCHEMAS_EP1.md`
- Scope: Cargo/workspace split + feature gating + @type schema contracts.
- Role in plan: implementation blueprint for repo structure.

3. `03_prompt_lineage_prov_obs.txt`
- Output target: `SPEC_LINEAGE_PROV_OBS_EP1.md`
- Scope: OpenLineage emission, PROV bundle, OBS automation policy.
- Role in plan: evidence substrate.

4. `04_prompt_platform_mocks.txt`
- Output target: `SPEC_PLATFORM_MOCKS_EP1.md`
- Scope: deterministic web/mobile/cli mock actors and ingest contracts.
- Role in plan: reproducible load generation.

5. `05_prompt_verifier.txt`
- Output target: `SPEC_VERIFIER_EP1.md`
- Scope: independent verifier CLI, replay checks, board-ready reports.
- Role in plan: external proof and go/no-go guard.

6. `06_prompt_eventstore_audit_gap11.txt`
- Output target: `SPEC_EVENTSTORE_AUDIT_GAP11.md`
- Scope: indexed audit queries, planner, pagination, reindex, performance tests.
- Role in plan: timeline/query reliability for TV/export/verifier.

7. `07_prompt_gaps_6_15.txt`
- Output target: `SPEC_GAPS_6_15_EP1.md`
- Scope: nonce persistence TTL (GAP-6) + stage secret rotation persistence chain (GAP-15).
- Role in plan: replay/auth hardening.

8. `08_prompt_governance_yaml_schemas.txt`
- Output target: `SPEC_GOVERNANCE_YAML_AND_SCHEMAS_EP1.md`
- Scope: `lab.governance.v0.yaml`, strict validator, schema contracts and optional codegen.
- Role in plan: runtime governance config authority.

9. `09_prompt_episode_runner.txt`
- Output target: `SPEC_EPISODE_RUNNER_EP1.md`
- Scope: one-command runner (`just episode-1`), full orchestration, finalize, verify, artifact pack.
- Role in plan: operational execution path.

## What Already Exists in UBL-CORE

Relevant current docs:
- `/Users/ubl-ops/UBL-CORE/docs/ops/EPISODE_1_PROTOCOL.md`
- `/Users/ubl-ops/UBL-CORE/docs/ops/EPISODE_TEMPLATE.md`
- `/Users/ubl-ops/UBL-CORE/docs/ops/EVENT_HUB.md`
- `/Users/ubl-ops/UBL-CORE/docs/ops/READINESS_EVIDENCE.md`

Current state:
- `EPISODE_1_PROTOCOL.md` is a concise normative outline.
- The prompt pack is deeper and implementation-oriented.
- There is overlap; avoid creating duplicate normative truth.

## Consolidation Rule

Keep one normative layer and one implementation layer:
- Normative: keep `/Users/ubl-ops/UBL-CORE/docs/ops/EPISODE_1_PROTOCOL.md` as short contract.
- Implementation: create an Episode 1 spec pack directory and place detailed specs there.

Recommended directory:
- `/Users/ubl-ops/UBL-CORE/docs/ops/episode-1/specs/`

## Execution Order (Pragmatic)

1. Author governance and contracts first
- `08` Governance YAML + validator.
- `02` Workspace split + schemas.

2. Close hardening gaps before orchestration
- `07` GAP-6/GAP-15.
- `06` EventStore GAP-11.

3. Finalize evidence pipeline
- `03` Lineage/PROV/OBS.
- `05` Verifier.

4. Enable deterministic workload generation
- `04` Platform mocks.

5. Stitch everything into one reproducible command
- `09` Episode runner.

6. Publish umbrella synthesis at the end
- `01` Full implementation synthesis (reflecting real decisions already made).

## Acceptance Definition for This Intake

This intake is complete when:
- All 9 prompt scopes are represented in tracked spec files.
- No duplicated normative contradictions with `EPISODE_1_PROTOCOL.md`.
- Tasklist points to the spec pack as implementation source.
- Every implementation PR references one prompt ID and one spec file section.

## Immediate Next Step

Create the spec pack skeleton and map each prompt to one file path under:
- `/Users/ubl-ops/UBL-CORE/docs/ops/episode-1/specs/`

