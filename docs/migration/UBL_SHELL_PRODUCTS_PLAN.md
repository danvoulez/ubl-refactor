# UBL Shell Products Plan

**Status**: proposed  
**Date**: 2026-02-21  
**Owner**: Core Runtime + Product Engineering

## Objective

Create a dedicated repository for product-specific shells that **always depend on UBL-CORE via GitHub git dependencies**, avoiding core code duplication.

## Repository Decision

Product shells repository is fixed as:

- `https://github.com/danvoulez/UBL-SHELLS`

Rationale:

- `UBL-CORE` remains OSS/public foundation in org scope.
- Product shells can evolve independently in the personal product stream.

## Architecture Rule

`UBL-CORE` is the single source of truth for runtime, policy, canon, receipt, and gate primitives.

`UBL-SHELLS` contains only:

- product-specific chips/schemas/adapters
- product API wrappers
- product UI/ops glue
- product deployment composition

No copy-paste of core crate code.

## Dependency Model (GitHub/Git)

In `UBL-SHELLS/Cargo.toml`, depend directly on `UBL-CORE` crates by git:

```toml
[workspace.dependencies]
ubl_runtime = { git = "https://github.com/LogLine-Foundation/UBL-CORE.git", package = "ubl_runtime", branch = "main" }
ubl_types = { git = "https://github.com/LogLine-Foundation/UBL-CORE.git", package = "ubl_types", branch = "main" }
ubl_receipt = { git = "https://github.com/LogLine-Foundation/UBL-CORE.git", package = "ubl_receipt", branch = "main" }
ubl_unc1 = { git = "https://github.com/LogLine-Foundation/UBL-CORE.git", package = "ubl_unc1", branch = "main" }
ubl_canon = { git = "https://github.com/LogLine-Foundation/UBL-CORE.git", package = "ubl_canon", branch = "main" }
ubl_chipstore = { git = "https://github.com/LogLine-Foundation/UBL-CORE.git", package = "ubl_chipstore", branch = "main" }
ubl_eventstore = { git = "https://github.com/LogLine-Foundation/UBL-CORE.git", package = "ubl_eventstore", branch = "main" }
ubl_kms = { git = "https://github.com/LogLine-Foundation/UBL-CORE.git", package = "ubl_kms", branch = "main" }
```

This ensures core updates are always available from GitHub.

## Core-Change Propagation (best practice)

To make “every core change influence shell repo” safely:

1. Keep git deps on `branch = "main"` in `UBL-SHELLS`.
2. Add `core-sync` workflow in `UBL-SHELLS` triggered by:
   - `repository_dispatch` from `UBL-CORE` on push to `main`
   - scheduled run (e.g. hourly) as fallback
3. In `core-sync`:
   - run `cargo update`
   - run full tests
   - open PR automatically when lockfile changes

This gives immediate influence while preserving controlled merges.

## CI Contract Between Repos

### In UBL-CORE (OSS repo)

- On every push to `main`, dispatch event to `danvoulez/UBL-SHELLS`:
  - event type: `ubl-core-updated`
  - payload: core commit SHA, branch, optional release tag
  - auth: repo secret `UBL_SHELLS_DISPATCH_TOKEN` (PAT with access to `danvoulez/UBL-SHELLS`)

### In UBL-SHELLS

- On `ubl-core-updated`:
  - update lockfile
  - run validation matrix
  - publish report/status

## Release Strategy

- `UBL-CORE` creates official releases via tag workflow.
- `UBL-SHELLS` tracks:
  - `main` for continuous integration with latest core
  - optional release branches pinned to core tags for production stability

## Initial Bootstrap Checklist (UBL-SHELLS)

1. Confirm repo `danvoulez/UBL-SHELLS`.
2. Initialize Rust workspace (product crates only).
3. Add git dependencies to `UBL-CORE` crates.
4. Add `core-sync` workflow (`repository_dispatch` + schedule).
5. Add policy rule: “No vendored core code”.
6. Add architecture docs linking to `UBL-CORE` references.
