# Open Source Scope

**Status**: active  
**Owner**: Repo Maintainer  
**Last reviewed**: 2026-02-22

## Objective

Define the boundary between the OSS `UBL-CORE` foundation and product-shell repositories.

## UBL-CORE (this repository)

In scope:

- deterministic runtime and pipeline
- canonicalization, receipt, and verification primitives
- gate service, MCP endpoint, and CLI
- core stores/connectors needed by runtime behavior
- core operational and security documentation

Out of scope:

- product-specific UX/APIs/business flows
- tenant-specific adapters and deployment wrappers
- product branding and distribution packaging

## Product Shells Repository

Product shells currently live at:

- [danvoulez/UBL-SHELLS](https://github.com/danvoulez/UBL-SHELLS)

Rules:

1. `UBL-SHELLS` depends on `UBL-CORE` via git dependencies.
2. Core crate code is not copied into shell repositories.
3. Core changes may trigger shell update workflows (dispatch + scheduled fallback).

## Release and Change Flow

1. `UBL-CORE` changes land on `main` only when they are core-level changes (protocol, security, conformance, compatibility, or critical performance/runtime fixes).
2. Product features and product UX/API flows should be implemented in shell/product repositories, not in `UBL-CORE`.
3. Core tags (`v*`) produce candidate releases and can be promoted to official releases.
4. `UBL-SHELLS` consumes updated core by git dependency updates and CI validation.

## Core Change Cadence

- During stabilization, `UBL-CORE` can change frequently.
- After baseline stabilization, `UBL-CORE` should move to controlled/episodic updates, not daily product iteration.
- Default operating mode:
  - `UBL-CORE`: conservative, high-rigor changes.
  - Product repos: rapid iteration on app-specific behavior.

## CI/CD Profiles

Two operational profiles are valid:

1. `public_core` (for OSS trust-facing core):
   - source of truth in Gitea
   - mirror to GitHub before publish
   - GitHub Actions for public gates/releases/attestation
   - signed source bundle to S3
   - deploy from S3
   - UBL receipts for publish/deploy events
2. `private_product` (for non-public product repos):
   - source of truth in Gitea
   - no mandatory GitHub mirror
   - signed source bundle to S3
   - deploy from S3
   - UBL receipts for publish/deploy events

Operational runbook: `docs/ops/GITEA_SOURCE_FLOW.md`.

## Economic Model Boundary

- Core protocol/runtime verification components are open-source.
- Paid value is expected in managed operations and enterprise integrations.
- See:
  - `COMMERCIAL-LICENSING.md`
  - `TRADEMARK_POLICY.md`
