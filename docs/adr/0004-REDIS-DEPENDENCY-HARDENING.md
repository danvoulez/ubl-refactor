# ADR 0004: Remove Unused Redis Dependency from UBL-CORE

**Status**: accepted
**Date**: 2026-02-21

## Context

`redis v0.25.4` was present in `UBL-CORE` manifests but not used by current core code paths.
Builds reported a Rust future-incompatibility warning (`never type fallback`) originating from that dependency.

`UBL-CORE` should keep only actively used dependencies in the OSS baseline to reduce supply-chain risk and CI noise.

## Decision

- Remove `redis` from:
  - workspace dependency declarations
  - `crates/ubl_chipstore` dependencies
- Keep current storage backends in core as implemented (`sled`, filesystem, S3 emulation path).
- Reintroduce Redis only when an implemented backend exists, behind an explicit feature gate and with a current supported version.

## Consequences

- Future-incompatibility warning from `redis v0.25.4` is eliminated in current core builds.
- Smaller dependency surface in OSS baseline.
- If Redis backend work resumes, it must include code, tests, and feature-gated dependency policy in the same change.
