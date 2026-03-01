# Compatibility Policy

**Status**: active  
**Owner**: Core Runtime  
**Last reviewed**: 2026-02-21

## Scope

Defines compatibility guarantees for:

- protocol/canonical behavior
- runtime APIs and receipts
- CLI and MCP surfaces
- migration between baseline releases

## Compatibility Levels

1. `Protocol`: strongest guarantee. Released protocol semantics are immutable per version id.
2. `API`: stable within major version, additive in minor, breaking only on major.
3. `Operational`: rollout/env defaults may evolve, but must be documented and traceable.

## Protocol Rules

- Existing protocol version behavior cannot be changed silently.
- Any breaking protocol semantic change requires a new version marker and migration notes.
- Deterministic verification for a fixed version must remain reproducible.

## API and CLI Rules

- Public API and CLI follow SemVer.
- Additive fields/commands are allowed in minor versions.
- Removed/renamed fields or command behavior changes require major bump (or explicit new endpoint/command path).

## MCP and Connector Rules

- Core MCP contract changes require RFC + changelog + compatibility note.
- Product-shell specific connector behavior must remain outside core compatibility promises unless promoted into core scope.

## Deprecation Policy

- Mark deprecations in docs/changelog before removal.
- Include replacement path and target removal milestone.
- Security exceptions are allowed, but must be called out in release notes.

## Verification Gate

Before official release:

1. Compatibility-impacting changes are listed in release notes.
2. Conformance/KAT suites for affected components pass.
3. Migration notes are available for operators and shell maintainers.

