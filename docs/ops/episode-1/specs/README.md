# Episode 1 Spec Pack

Status: active
Date: 2026-02-23
Owner: UBL-CORE engineering

This directory is the implementation layer for Episode 1.
Normative contract remains in:
- `/Users/ubl-ops/UBL-CORE/docs/ops/EPISODE_1_PROTOCOL.md`

## Prompt-to-Spec Mapping

- 01 -> `SPEC_IMPLEMENTACAO_EP1.md` (materialized)
- 02 -> `SPEC_WORKSPACE_E_SCHEMAS_EP1.md` (materialized)
- 03 -> `SPEC_LINEAGE_PROV_OBS_EP1.md` (materialized)
- 04 -> `SPEC_PLATFORM_MOCKS_EP1.md` (materialized)
- 05 -> `SPEC_VERIFIER_EP1.md` (materialized)
- 06 -> `SPEC_EVENTSTORE_AUDIT_GAP11.md` (materialized)
- 07 -> `SPEC_GAPS_6_15_EP1.md` (materialized)
- 08 -> `SPEC_GOVERNANCE_YAML_AND_SCHEMAS_EP1.md` (materialized)
- 09 -> `SPEC_EPISODE_RUNNER_EP1.md` (materialized)

## Execution Order

1. `SPEC_GOVERNANCE_YAML_AND_SCHEMAS_EP1.md`
2. `SPEC_WORKSPACE_E_SCHEMAS_EP1.md`
3. `SPEC_GAPS_6_15_EP1.md`
4. `SPEC_EVENTSTORE_AUDIT_GAP11.md`
5. `SPEC_LINEAGE_PROV_OBS_EP1.md`
6. `SPEC_VERIFIER_EP1.md`
7. `SPEC_PLATFORM_MOCKS_EP1.md`
8. `SPEC_EPISODE_RUNNER_EP1.md`
9. `SPEC_IMPLEMENTACAO_EP1.md`

## Authoring Rules

- Fail-closed defaults only.
- Deterministic semantics must be explicit.
- Every implementation PR references: prompt id + section id in one spec file.
- Do not fork normative behavior from `EPISODE_1_PROTOCOL.md`.
