# ChatGPT Connector Onboarding

This runbook onboards a ChatGPT connector into UBL and mints a scoped `ubl/token`
for `Authorization: Bearer <token_id>`.

The gate enforces write auth and world scope:

- token writes are allowed only for token `@world` (or child path).
- cross-world writes are denied with `POLICY_DENIED`.

## Prerequisites

- Running gate (`/healthz` is `ok`).
- `jq`, `curl`, and either:
  - installed `ublx`, or
  - local source tree where `cargo run -p ubl_cli -- ...` works.

## Minimal mode (join existing app/tenant)

Use this when `a/<app>` already exists.

```bash
cd /Users/ubl-ops/UBL-CORE
scripts/onboard_chatgpt_connector.sh \
  --gate https://api.ubl.agency \
  --app chip-registry \
  --tenant logline
```

Outputs:

- generated chips + responses under `artifacts/onboarding-chatgpt-<ts>/`
- `summary.json` containing:
  - connector DID
  - token id/cid/scope/expiry
  - ready auth header

Important:

- For onboarding dependencies (`creator_cid`, `user_cid`, `tenant_cid`) use the
  canonical CID computed from submitted chip JSON (`ublx cid <chip.json>`).
- Do not rely on `response.chain[0]` for dependency linking.

## Full bootstrap mode (create app/tenant/membership too)

Use this when the app/tenant do not exist yet.

```bash
cd /Users/ubl-ops/UBL-CORE
scripts/onboard_chatgpt_connector.sh \
  --full-bootstrap \
  --founder-signing-key-hex "$FOUNDER_SIGNING_KEY_HEX" \
  --gate https://api.ubl.agency \
  --app chip-registry \
  --tenant logline
```

Full mode issues signed capabilities (`@cap`) for:

- `registry:init` (for `ubl/app` and first `ubl/user`)
- `membership:grant` (for `ubl/membership`)

## Use token in ChatGPT Actions / MCP

From `summary.json`:

- `token.id` -> `Bearer <token.id>`
- `worlds.tenant` -> expected write scope

Recommended connector settings:

- Actions API base: `https://api.ubl.agency`
- MCP URL: `https://api.ubl.agency/mcp/rpc`
- Auth header: `Authorization: Bearer <token_id>`

## Security notes

- If connector DID was auto-generated, private key seed is written to:
  - `artifacts/onboarding-chatgpt-<ts>/connector.did.json`
- Move that file to secure storage immediately and remove local plaintext copy.
- Rotate tokens with new `ubl/token` chips and revoke old ones with `ubl/revoke`.
