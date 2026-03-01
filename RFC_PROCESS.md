# RFC Process

**Status**: active  
**Owner**: Repo Maintainer  
**Last reviewed**: 2026-02-21

## Purpose

Define how high-impact changes are proposed, reviewed, and accepted for `UBL-CORE`.

## When an RFC Is Required

Open an RFC before implementation when a change affects one or more of:

- protocol semantics or canonicalization behavior
- receipt verification rules or cryptographic trust boundaries
- deterministic pipeline stages/invariants
- public API contracts (`/openapi.json`, MCP RPC shape, CLI compatibility promises)
- compatibility/deprecation policy across major/minor lines
- scope boundary between `UBL-CORE` and product-shell repos

Small refactors, bug fixes without contract impact, and non-normative docs updates do not require RFCs.

## RFC Lifecycle

1. `draft`: problem statement, options, proposed decision, migration impact.
2. `review`: open for maintainer/community comments.
3. `accepted`: decision locked; implementation can merge.
4. `rejected` or `withdrawn`: closed with rationale.
5. `superseded`: replaced by a newer accepted RFC.

## Repository Layout

- Index: `docs/rfc/README.md`
- New RFC files: `docs/rfc/NNNN-short-title.md`
- Template: `docs/rfc/TEMPLATE.md`

## Merge Rules

- No merge of behavior-changing code before RFC status is `accepted` (except emergency security hotfixes).
- PRs implementing accepted RFCs must reference the RFC id.
- If implementation deviates materially, update RFC first.

## Relationship With ADRs

- RFCs capture forward-looking proposals and public compatibility impact.
- ADRs capture architecture decisions after commitment.
- Accepted RFCs may result in one or more ADRs in `docs/adr/`.

