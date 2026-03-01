# Versioning Policy

**Status**: active  
**Owner**: Core Runtime  
**Last reviewed**: 2026-02-21

## Goals

- keep protocol behavior auditable and predictable
- avoid silent breaking changes in OSS consumers
- keep release tags and crate versions coherent

## Repository Release Tags

- Tag pattern for baseline releases: `vX.Y.Z-core-baseline`
- Optional candidates/previews may use suffixes (for example `-rc1`).
- Official/latest release promotion is done via `Release From Tag` workflow.

## Crate Versioning

`UBL-CORE` crates follow SemVer:

- `MAJOR`: breaking API or contract changes
- `MINOR`: backward-compatible features
- `PATCH`: backward-compatible fixes

For public crates, semver checks in CI are treated as gate signals for release readiness.

## Protocol and Spec Versioning

- Canonical/protocol artifacts carry explicit versions (`@ver`, schema version, opcode/version tables).
- Breaking protocol behavior requires a new protocol/spec version, not silent mutation of existing version semantics.
- Compatibility pointers may exist, but canonical behavior for each version is immutable once released.

## Deprecation Window

- Deprecations must be documented in changelog and compatibility docs before removal.
- Default policy: at least one minor release with deprecation notice before removal, unless urgent security risk requires faster action.

## Required Update Surface for Breaking Changes

When a breaking change is accepted:

1. Update crate versions and changelog.
2. Update `COMPATIBILITY.md`.
3. Update affected canonical docs/spec references.
4. Provide migration notes in release body and docs.

