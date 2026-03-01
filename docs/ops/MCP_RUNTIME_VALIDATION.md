# MCP Runtime Validation

**Status**: active  
**Owner**: Core Runtime + Ops  
**Last reviewed**: 2026-02-22

## Objective

Provide a reproducible validation that UBL MCP is live and supports read/write operations, not only manifest discovery.

## Validation Target

- Gate base URL: `http://127.0.0.1:4000`
- Pipeline: `KNOCK -> WA -> CHECK -> TR -> WF`

## Public MCP Endpoints (Current)

Primary public host (active):

- [`https://api.ubl.agency/mcp/manifest`](https://api.ubl.agency/mcp/manifest) -> `200`
- [`https://api.ubl.agency/.well-known/webmcp.json`](https://api.ubl.agency/.well-known/webmcp.json) -> `200`
- [`https://api.ubl.agency/mcp/rpc`](https://api.ubl.agency/mcp/rpc) -> `200` on GET (`text/event-stream`) and `200` on POST (`application/json`)
- [`https://api.ubl.agency/mcp/sse`](https://api.ubl.agency/mcp/sse) -> `200` on GET (`text/event-stream`)

Optional dedicated MCP host (not configured yet):

- [`https://mcp.ubl.agency/mcp/manifest`](https://mcp.ubl.agency/mcp/manifest) -> `404`

Operational note:

- A dedicated `mcp.ubl.agency` DNS/tunnel route is optional.
- MCP is already publicly usable through `api.ubl.agency`.

## Validation Checklist

### 1) Runtime health

```bash
curl -s http://127.0.0.1:4000/healthz
```

Expected: JSON with `"status":"ok"`.

Observed (2026-02-22):

```json
{"status":"ok","system":"ubl-core","pipeline":"KNOCK->WA->CHECK->TR->WF"}
```

### 2) HTTP write path (API)

```bash
curl -s -X POST http://127.0.0.1:4000/v1/chips \
  -H 'content-type: application/json' \
  --data '{"@type":"ubl/document","@id":"probe-mcp-vs-api","@ver":"1.0","@world":"a/chip-registry/t/logline","title":"probe"}' \
  | jq '{receipt_cid,decision,reason}'
```

Observed `receipt_cid` example:

- `b3:9064075634980e7a70ec927c9cd22643fb06a1e48d53e21d1e1059dd2d4f1493`

### 3) MCP discovery endpoints

```bash
curl -s http://127.0.0.1:4000/mcp/manifest | jq '.name,.version,.tools|length'
curl -s http://127.0.0.1:4000/.well-known/webmcp.json | jq '.name,.version,.tools|length'
```

Observed:

- `/mcp/manifest` -> `200`
- `/.well-known/webmcp.json` -> `200`

Note: `GET /mcp/rpc` is SSE bootstrap (`text/event-stream`) and `POST /mcp/rpc` is JSON-RPC.

### 4) MCP SSE transport bootstrap

```bash
curl -i --max-time 7 https://api.ubl.agency/mcp/rpc
```

Expected:

- `HTTP 200`
- `content-type: text/event-stream`
- first event `mcp.ready`

### 5) MCP JSON-RPC tool listing

```bash
curl -s -X POST http://127.0.0.1:4000/mcp/rpc \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":"1","method":"tools/list","params":{}}' \
  | jq '.result.tools | map(.name)'
```

Observed key tools include:

- `ubl.deliver`
- `ubl.chip.submit`
- `ubl.query`
- `ubl.receipt`
- `ubl.verify`
- `registry.listTypes`

### 6) MCP write path (`tools/call`)

```bash
curl -s -X POST http://127.0.0.1:4000/mcp/rpc \
  -H 'content-type: application/json' \
  --data '{
    "jsonrpc":"2.0",
    "id":"call-1",
    "method":"tools/call",
    "params":{
      "name":"ubl.deliver",
      "arguments":{
        "chip":{
          "@type":"ubl/document",
          "@id":"probe-mcp-write-20260222",
          "@ver":"1.0",
          "@world":"a/chip-registry/t/logline",
          "title":"mcp write probe"
        }
      }
    }
  }' | jq '.result.content[0].text'
```

Observed: successful `ubl/response` payload containing:

- `decision: "Allow"`
- `receipt_cid: "b3:6bd05a196c96c1c6789d14522b9ca8767213ac5e0c8aa2157cc5ce9ddd63f1ae"`

### 7) ChatGPT Custom App connector settings

Use these values in ChatGPT "New App":

- **Name**: `UBLx`
- **MCP Server URL**: `https://api.ubl.agency/mcp/rpc`
- **Authentication**: `None` (for current public validation)

Notes:

- `https://api.ubl.agency/mcp/manifest` remains valid for discovery docs.
- Connector handshake requires SSE; using `/mcp/rpc` avoids the previous mismatch (`expected text/event-stream`).
- Cloudflare challenge pages must not appear on `/mcp/*` and `/.well-known/webmcp.json`; if they appear, edge policy is regressed.

## Write Security Model (Current)

- Read operations remain public (`tools/list`, `ubl.query`, `ubl.receipt`, `ubl.verify`).
- Write operations (`ubl.deliver` / `POST /v1/chips`) support:
  - `Authorization: Bearer <token_id>` resolved against `ubl/token` chips (preferred).
  - `X-API-Key` (optional fallback/break-glass).
- Bearer tokens are bound to token `@world` scope:
  - token can write only to same `@world` or child prefix of that world.
  - cross-world writes return `POLICY_DENIED`.
- Public write lane can remain open for onboarding/public registry worlds by policy.

Recommended:

- Use `ubl/token` for machine clients (ChatGPT Actions/MCP connectors).
- Keep API keys only for emergency bootstrap or break-glass operations.
- For operational onboarding flow, use:
  - `docs/ops/CHATGPT_CONNECTOR_ONBOARDING.md`
  - `scripts/onboard_chatgpt_connector.sh`

## Conclusion

MCP is operational in practice:

- discovery manifests are live
- JSON-RPC transport is live
- tool listing works
- write path through MCP (`ubl.deliver`) succeeds and produces receipts

What remains external to runtime:

- client-side connector setup (e.g., ChatGPT app connector/config, auth wiring, scopes).
