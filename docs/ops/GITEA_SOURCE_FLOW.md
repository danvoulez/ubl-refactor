# Gitea Source Flow

Canonical source flow for UBL-CORE:

1. Git source of truth in Gitea
2. Mirror to GitHub (public-core profile)
3. Signed source bundles in versioned S3/MinIO
4. Operational chips emitted through `ublx submit` into UBL gate

## Configure

```bash
cp /Users/ubl-ops/UBL-CORE/ops/source_flow.env.example /Users/ubl-ops/UBL-CORE/ops/source_flow.env
# edit values
```

## Commands

```bash
# inspect active profile and gates
/Users/ubl-ops/UBL-CORE/scripts/source_flow.sh --env /Users/ubl-ops/UBL-CORE/ops/source_flow.env profile

# configure gitea remote + push default
/Users/ubl-ops/UBL-CORE/scripts/source_flow.sh --env /Users/ubl-ops/UBL-CORE/ops/source_flow.env init-remote

# push current HEAD to gitea
/Users/ubl-ops/UBL-CORE/scripts/source_flow.sh --env /Users/ubl-ops/UBL-CORE/ops/source_flow.env push --branch main

# mirror same HEAD to github (required for public_core before publish)
/Users/ubl-ops/UBL-CORE/scripts/source_flow.sh --env /Users/ubl-ops/UBL-CORE/ops/source_flow.env mirror-github --branch main

# publish signed git bundle to S3 + emit ops/source.publish.v1 via ublx
/Users/ubl-ops/UBL-CORE/scripts/source_flow.sh --env /Users/ubl-ops/UBL-CORE/ops/source_flow.env publish

# deploy latest signed bundle + emit ops/source.deploy.v1 via ublx
/Users/ubl-ops/UBL-CORE/scripts/source_flow.sh --env /Users/ubl-ops/UBL-CORE/ops/source_flow.env deploy
```

## UBL CLI Requirement

`source_flow.sh` emits pipeline chips with `ublx submit`.

Resolution order:

1. `SOURCE_UBLX_BIN` (default `ublx`)
2. Fallback `cargo run -p ubl_cli -- submit` when `SOURCE_UBLX_CARGO_FALLBACK=true`

If no CLI is available and fallback is disabled, emission fails closed for that step.

## Profiles

- `public_core`
  - requires GitHub mirror at the same HEAD before publish
  - records local trust binary hash (LAB 256 / LAB 512 style)
- `private_product`
  - no mandatory GitHub mirror
  - still supports signed S3 source bundles and UBL receipts
