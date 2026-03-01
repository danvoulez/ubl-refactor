# Task Orchestration Protocol (Tasklist via UBL)

**Status**: draft  
**Owner**: Core Runtime + Platform Engineering  
**Last reviewed**: 2026-02-24

## Objective

Execute the program tasklist using the UBL pipeline itself.
Every task transition is a chip and every transition generates receipt evidence.

## Scope

This protocol defines:
- canonical task event type
- lifecycle transitions
- dependency enforcement in CHECK
- evidence linking for completion
- operator/NOC visibility via event stream

It does not replace business policy or release policy; it operationalizes them.

## Canonical task event chip

- Chip type: `task.lifecycle.event.v1`
- Envelope anchors required: `@id`, `@type`, `@ver`, `@world`
- Schema: `schemas/task.lifecycle.event.v1.json`

## Lifecycle model

Allowed task states:
- `open`
- `blocked`
- `in_progress`
- `done`
- `canceled`

Allowed transitions:
- `open -> in_progress`
- `open -> blocked`
- `blocked -> in_progress`
- `in_progress -> done`
- `in_progress -> blocked`
- `open -> canceled`
- `blocked -> canceled`
- `in_progress -> canceled`

## CHECK-stage enforcement

CHECK MUST reject events when:
- transition is not in allowed set
- dependencies are unresolved
- required evidence is missing for `done`
- actor lacks capability to transition target task

CHECK SHOULD include reason codes in error body:
- `TASK_INVALID_TRANSITION`
- `TASK_DEPENDENCY_UNRESOLVED`
- `TASK_EVIDENCE_REQUIRED`
- `TASK_CAPABILITY_DENIED`

## Required fields by state

For `blocked`:
- `blocker_code`
- `notes`

For `done`:
- `evidence` (at least one receipt CID or artifact path)

For `canceled`:
- `notes`

## Evidence model

A task can be completed only when evidence is linked:
- receipt CID(s) from execution
- conformance/test artifacts
- document/path references

Minimum requirement for `done`:
- one item in `evidence`

## Event stream and observability

Task orchestration consumers SHOULD subscribe to gate event stream and render:
- current status by task id
- blockers by code
- completion evidence links
- burn-down by track

## Chip examples

### Open a task

```json
{
  "@id": "task-L01-open-2026-02-24T21:00:00Z",
  "@type": "task.lifecycle.event.v1",
  "@ver": "v1",
  "@world": "ubl.platform.test",
  "task_id": "L-01",
  "track": "track-2",
  "title": "Publish NRF-1.1 normative spec",
  "state": "open",
  "depends_on": [],
  "evidence": [],
  "notes": "Created from Program Tasklist v2",
  "actor": { "did": "did:key:z...", "role": "platform" }
}
```

### Mark task as done

```json
{
  "@id": "task-L01-done-2026-02-28T10:10:00Z",
  "@type": "task.lifecycle.event.v1",
  "@ver": "v1",
  "@world": "ubl.platform.test",
  "task_id": "L-01",
  "track": "track-2",
  "title": "Publish NRF-1.1 normative spec",
  "state": "done",
  "depends_on": [],
  "evidence": [
    "b3:receipt_cid_...",
    "docs/canon/NRF-1.1.md"
  ],
  "notes": "Spec + conformance evidence linked",
  "actor": { "did": "did:key:z...", "role": "platform" }
}
```

## CLI integration (ublx)

Example:

```bash
ublx submit --type task.lifecycle.event.v1 --body @artifacts/tasks/L-01.done.json
```

Recommended automation:
- generate task chips from `TASKLIST.md`
- submit through gate
- capture receipt CIDs in `artifacts/tasks/`
- bootstrap first lifecycle events with `scripts/task_orchestrate.sh`

## Mapping to Program Tasklist

- Tracks in `TASKLIST.md` map to `track` field.
- Lacunas `L-*` map to `task_id`.
- Critical path maps to dependency graph in `depends_on`.

## Adoption plan

1. Start with `L-*` lacuna tasks only.
2. Enforce CHECK validations for dependencies and evidence.
3. Expand to all tracks after first successful weekly cycle.
4. Add dashboard panel for task orchestration status.

## Definition of done for this protocol

- schema active and versioned
- at least 5 real task transitions executed via chips
- one blocked -> in_progress -> done path with receipts
- task view available in observability app/event stream
