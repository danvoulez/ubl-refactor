# LAB 512 Readiness Evidence Log

Last updated: 2026-02-24T03:13:34Z

## Phase 3 — Bootstrap Pipeline Hardening

- Final review of `scripts/forever_bootstrap.sh` for LAB 512 execution path.
  - Command: `sed -n '1,240p' /Users/ubl-ops/UBL-CORE/scripts/forever_bootstrap.sh`
  - Command: `sed -n '820,1180p' /Users/ubl-ops/UBL-CORE/scripts/forever_bootstrap.sh`
  - Command: `rg -n "bundle|upload|receipt|report" /Users/ubl-ops/UBL-CORE/scripts/forever_bootstrap.sh`
  - Evidence: final bundle generation/upload + final report receipt controls are enforced by env gates and strict flags (see `UBL_FINAL_BUNDLE_ENABLE`, `UBL_FINAL_BUNDLE_UPLOAD_ENABLE`, `UBL_FINAL_BUNDLE_REQUIRE_REMOTE`, `UBL_FINAL_REPORT_RECEIPT_ENABLE`, `UBL_FINAL_REPORT_RECEIPT_REQUIRE`).

- Final review of `scripts/host_lockdown.sh` for service-user model (`ubl-service`).
  - Command: `sed -n '1,200p' /Users/ubl-ops/UBL-CORE/scripts/host_lockdown.sh`

- Final review of `scripts/workzone_cleanup.sh` for safe cleanup boundaries.
  - Command: `sed -n '1,200p' /Users/ubl-ops/UBL-CORE/scripts/workzone_cleanup.sh`

- Ensure final bundle generation/upload is mandatory where required.
  - Command: `sed -n '1,200p' /Users/ubl-ops/UBL-CORE/ops/forever_bootstrap.env`
  - Evidence: `UBL_FINAL_BUNDLE_ENABLE=true`, `UBL_FINAL_BUNDLE_UPLOAD_ENABLE=true`, `UBL_FINAL_BUNDLE_REQUIRE_REMOTE=true`.

- Ensure final local checks emit canonical report + receipt.
  - Command: `sed -n '1,200p' /Users/ubl-ops/UBL-CORE/ops/forever_bootstrap.env`
  - Evidence: `UBL_FINAL_REPORT_RECEIPT_ENABLE=true`, `UBL_FINAL_REPORT_RECEIPT_REQUIRE=true`.

## Phase 5 — Quality Gates Before Promotion

- `make contract` passes.
  - Command: `cd /Users/ubl-ops/UBL-CORE && make contract`
  - Evidence: `artifacts/contract/latest.json`, `artifacts/contract/latest.md`.

- `make conformance` passes.
  - Command: `cd /Users/ubl-ops/UBL-CORE && make conformance`
  - Evidence: `artifacts/conformance/latest.json`, `artifacts/conformance/latest.md`.

- CI `WF` gate requires CI run ID.
  - Command: `rg -n "WF" /Users/ubl-ops/UBL-CORE/.github/workflows`
  - Evidence: `WF` gate defined in `.github/workflows/ci.yml` and depends on CI execution; no CI run available in this environment.

- Reproducibility/attestation checks not executed for target commit.
  - Command: `rg -n "reproducibility|attestation" /Users/ubl-ops/UBL-CORE/docs/ops`
  - Evidence: reproducibility/attestation references exist (for example `docs/ops/FOREVER_BOOTSTRAP.md`, `docs/ops/INCIDENT_RUNBOOK.md`), but no runbook or recorded check for the target commit is present.

- Promotion checklist document/signoff not present.
  - Command: `rg -n "promotion checklist|LAB 256 -> LAB 512" /Users/ubl-ops/UBL-CORE/docs/ops`
  - Evidence: no promotion checklist doc or signoff log found in repo.

- Phase 5 doc scan (WF workflow, reproducibility/attestation, promotion checklist).
  - Command: `bash -lc 'set -euo pipefail; ts=$(date -u +%Y-%m-%dT%H:%M:%SZ); out=/Users/ubl-ops/UBL-CORE/artifacts/readiness/2026-02-24_phase5_review.txt; mkdir -p /Users/ubl-ops/UBL-CORE/artifacts/readiness; { echo "Timestamp (UTC): $ts"; echo "PWD: $(pwd)"; echo; echo "## WF workflow references"; rg -n "WF" /Users/ubl-ops/UBL-CORE/.github/workflows || true; echo; echo "## Reproducibility/attestation references"; rg -n "reproducibility|attestation" /Users/ubl-ops/UBL-CORE/docs/ops || true; echo; echo "## Promotion checklist references"; rg -n "promotion checklist|LAB 256 -> LAB 512" /Users/ubl-ops/UBL-CORE/docs/ops || true; } > "$out"; echo "$out"'`
  - Evidence: `/Users/ubl-ops/UBL-CORE/artifacts/readiness/2026-02-24_phase5_review.txt`.

## Phase 1 — Architecture Validation for LAB 512

- Phase 1 doc scan (topology, canary/dual-plane, handoff, failure/rollback).
  - Command: `bash -lc 'set -euo pipefail; ts=$(date -u +%Y-%m-%dT%H:%M:%SZ); out=/Users/ubl-ops/UBL-CORE/artifacts/readiness/2026-02-24_phase1_review.txt; mkdir -p /Users/ubl-ops/UBL-CORE/artifacts/readiness; { echo "Timestamp (UTC): $ts"; echo "PWD: $(pwd)"; echo; echo "## Topology references"; rg -n "control plane|data plane|topology" /Users/ubl-ops/UBL-CORE/docs/ops || true; echo; echo "## EPISODE_1_PROTOCOL.md (1-80)"; sed -n "1,80p" /Users/ubl-ops/UBL-CORE/docs/ops/EPISODE_1_PROTOCOL.md; echo; echo "## Canary/dual-plane references"; rg -n "canary|dual-plane" /Users/ubl-ops/UBL-CORE/docs/ops || true; echo; echo "## BOOTSTRAP_FINAL_TEXT.md (20-200)"; sed -n "20,200p" /Users/ubl-ops/UBL-CORE/docs/ops/BOOTSTRAP_FINAL_TEXT.md; echo; echo "## Handoff/replication references"; rg -n "handoff|replic|sync" /Users/ubl-ops/UBL-CORE/docs/ops || true; echo; echo "## Failure/rollback references"; rg -n "failure|rollback|recover" /Users/ubl-ops/UBL-CORE/docs/ops || true; } > "$out"; echo "$out"'`
  - Evidence: `/Users/ubl-ops/UBL-CORE/artifacts/readiness/2026-02-24_phase1_review.txt`.

## Phase 2 — Security and Identity Ceremony

- Phase 2 doc scan (ceremony, trust anchors, identity, break-glass, secret handling).
  - Command: `bash -lc 'set -euo pipefail; ts=$(date -u +%Y-%m-%dT%H:%M:%SZ); out=/Users/ubl-ops/UBL-CORE/artifacts/readiness/2026-02-24_phase2_review.txt; mkdir -p /Users/ubl-ops/UBL-CORE/artifacts/readiness; { echo "Timestamp (UTC): $ts"; echo "PWD: $(pwd)"; echo; echo "## Ceremony/key birth references"; rg -n "ceremony|key birth|machine birth" /Users/ubl-ops/UBL-CORE/docs/ops || true; echo; echo "## Trust anchor/attestation references"; rg -n "trust anchor|attestation" /Users/ubl-ops/UBL-CORE/docs/ops || true; echo; echo "## Authorship/identity/ingress references"; rg -n "authorship|identity|ingress" /Users/ubl-ops/UBL-CORE/docs/ops || true; echo; echo "## Break-glass/operator/admin references"; rg -n "break-glass|operator/admin" /Users/ubl-ops/UBL-CORE/docs/ops || true; echo; echo "## Secret handling/leak references"; rg -n "secret|leak|artifact" /Users/ubl-ops/UBL-CORE/docs/ops || true; } > "$out"; echo "$out"'`
  - Evidence: `/Users/ubl-ops/UBL-CORE/artifacts/readiness/2026-02-24_phase2_review.txt`.

## Phase 0-6 — Blocker Assessment Snapshot (LAB 256-safe)

- Snapshot of current repo state and supporting documentation references.
  - Command: `bash -lc "mkdir -p /Users/ubl-ops/UBL-CORE/artifacts/readiness && { echo \\\"Timestamp (UTC): $(date -u +%Y-%m-%dT%H:%M:%SZ)\\\"; echo \\\"PWD: $(pwd)\\\"; echo; echo \\\"## Git state\\\"; git -C /Users/ubl-ops/UBL-CORE rev-parse --abbrev-ref HEAD; git -C /Users/ubl-ops/UBL-CORE rev-parse HEAD; git -C /Users/ubl-ops/UBL-CORE status -sb; echo; echo \\\"## Ops docs index\\\"; ls -la /Users/ubl-ops/UBL-CORE/docs/ops; echo; echo \\\"## Control/data plane references\\\"; rg -n \\\"control plane|data plane|dual-plane|handoff|rollback|topology|canary\\\" /Users/ubl-ops/UBL-CORE/docs/ops || true; echo; echo \\\"## EPISODE_1_PROTOCOL.md (excerpt)\\\"; sed -n '1,200p' /Users/ubl-ops/UBL-CORE/docs/ops/EPISODE_1_PROTOCOL.md; echo; echo \\\"## BOOTSTRAP_FINAL_TEXT.md (excerpt)\\\"; sed -n '1,200p' /Users/ubl-ops/UBL-CORE/docs/ops/BOOTSTRAP_FINAL_TEXT.md; echo; echo \\\"## Cloudflare docs\\\"; sed -n '1,160p' /Users/ubl-ops/UBL-CORE/docs/ops/CLOUDFLARE_EDGE_BASELINE.md; echo; sed -n '1,200p' /Users/ubl-ops/UBL-CORE/docs/ops/CLOUDFLARE_TUNNEL_GO_LIVE.md; echo; echo \\\"## Security/identity references\\\"; rg -n \\\"ceremony|attestation|break-glass|trust anchor|authorship|identity\\\" /Users/ubl-ops/UBL-CORE/docs/ops || true; echo; echo \\\"## Reproducibility/attestation references\\\"; rg -n \\\"reproducibility|attestation|promotion checklist|LAB 256|LAB 512\\\" /Users/ubl-ops/UBL-CORE/docs/ops || true; } > /Users/ubl-ops/UBL-CORE/artifacts/readiness/2026-02-23_phase0-6_blockers.txt"`
  - Evidence: `/Users/ubl-ops/UBL-CORE/artifacts/readiness/2026-02-23_phase0-6_blockers.txt`.

## Phase 4 — Edge and Exposure Hardening (Cloudflare + DNS)

- Cloudflare credentials and tunnel/rate-limit automation unavailable in this environment.
  - Command: `env | rg -i "CLOUDFLARE|UBL_CLOUDFLARE" || true`
  - Evidence: No Cloudflare-related environment variables detected; requires operator-provided API token + tunnel IDs.
  - Update (2026-02-23T14:44:58Z): rechecked env; still no Cloudflare tokens or tunnel IDs available in session.
  - Update (2026-02-23T14:50:19Z): rechecked env; only `__CF_USER_TEXT_ENCODING` present, no API token/tunnel IDs available.
  - Update (2026-02-23T20:45:20Z): rechecked env; no Cloudflare tokens or tunnel IDs available in session.
  - Update (2026-02-23T20:50:15Z): rechecked env; only `__CF_USER_TEXT_ENCODING` present, no Cloudflare API token/tunnel IDs available.
  - Update (2026-02-24T02:44:47Z): rechecked env; no Cloudflare tokens or tunnel IDs available in session.
  - Manual steps (operator):
    - Confirm Cloudflare Access policy is active, then set in bootstrap env:
      - `UBL_CLOUDFLARE_ENABLE=true`
      - `UBL_CLOUDFLARE_ACCESS_POLICY_CONFIRMED=true`
    - Create/confirm tunnel(s) in Zero Trust and note tunnel IDs for `ubl.agency`, `api.ubl.agency`, `logline.world`.
    - Create proxied `CNAME` records to `<TUNNEL_ID>.cfargotunnel.com` for each hostname.
    - Configure rate limit rules for `/v1/chips` and `/v1/receipts`; record rule IDs in:
      - `UBL_CLOUDFLARE_RATE_LIMIT_RULES` or `${UBL_BASE_DIR}/state/cloudflare_rate_limit.json`.

- Cloudflare Access app/policy prerequisites require operator confirmation.
  - Command: `sed -n '1,200p' /Users/ubl-ops/UBL-CORE/docs/ops/CLOUDFLARE_EDGE_BASELINE.md`
  - Command: `sed -n '1,240p' /Users/ubl-ops/UBL-CORE/docs/ops/CLOUDFLARE_TUNNEL_GO_LIVE.md`
  - Evidence: checklist requires Access app/policy creation and confirmation before setting `UBL_CLOUDFLARE_ACCESS_POLICY_CONFIRMED=true`; no Cloudflare account context available in this environment.
  - Update (2026-02-24T02:50:22Z): no Cloudflare credentials present in environment (`CLOUDFLARE*`/`CF_*`), so Access app/policy prerequisites cannot be verified or updated.

- Tunnel + DNS automation path requires Cloudflare account access and tunnel IDs.
  - Command: `sed -n '1,240p' /Users/ubl-ops/UBL-CORE/docs/ops/CLOUDFLARE_TUNNEL_GO_LIVE.md`
  - Evidence: go-live steps require Zero Trust tunnel creation, Access policy setup, and tunnel ID mapping for `ubl.agency`, `api.ubl.agency`, `logline.world`.
  - Update (2026-02-24T02:50:22Z): no Cloudflare API token or tunnel IDs available in environment; tunnel/DNS changes could not be applied.

- Edge rate limiting rule registry not populated.
  - Command: `rg -n "rate limit|rule ID" /Users/ubl-ops/UBL-CORE/docs/ops`
  - Evidence: docs specify storing rule IDs in `UBL_CLOUDFLARE_RATE_LIMIT_RULES` or `${UBL_BASE_DIR}/state/cloudflare_rate_limit.json`, but no IDs available in this environment.
  - Update (2026-02-24T02:50:22Z): no Cloudflare credentials available to query or confirm rate limit rule IDs.

- Receipt URL model confirmation requires operator signoff.
  - Command: `rg -n "receipt model|logline.world|/r#ubl:v1" /Users/ubl-ops/UBL-CORE/docs/ops`
  - Evidence: `docs/ops/BOOTSTRAP_FINAL_TEXT.md` defines `https://logline.world/r#ubl:v1:<token>` as the public receipt URL; no operator confirmation recorded.

## MCP + API Endpoint Validation

- Local runtime health check failed (no local gate running).
  - Command: `curl -sS --max-time 5 http://127.0.0.1:4000/healthz`
  - Evidence: connection refused (2026-02-23T14:44:58Z).
  - Update (2026-02-23T20:45:20Z): connection refused; gate still not running locally.
  - Update (2026-02-23T20:50:15Z): connection refused; gate still not running locally.
  - Update (2026-02-24T02:44:47Z): connection refused; gate still not running locally.
  - Manual steps (operator):
    - Start the gate locally, then re-run:
      - `curl -sS http://127.0.0.1:4000/healthz`
      - `curl -sS http://127.0.0.1:4000/mcp/manifest`
      - `curl -sS http://127.0.0.1:4000/.well-known/webmcp.json`
      - `curl -sS -X POST http://127.0.0.1:4000/mcp/rpc -H 'content-type: application/json' --data '{"jsonrpc":"2.0","id":"1","method":"tools/list","params":{}}'`
      - `curl -sS -X POST http://127.0.0.1:4000/v1/chips -H 'content-type: application/json' --data '{"@type":"ubl/document","@id":"probe-mcp-vs-api","@ver":"1.0","@world":"a/chip-registry/t/logline","title":"probe"}'`

- Public endpoint validation blocked by restricted DNS/network access in this environment.
  - Command: `curl -I -sS --max-time 10 https://ubl.agency/healthz`
  - Command: `curl -I -sS --max-time 10 https://api.ubl.agency/healthz`
  - Command: `curl -I -sS --max-time 10 https://api.ubl.agency/mcp/manifest`
  - Command: `curl -I -sS --max-time 10 https://api.ubl.agency/.well-known/webmcp.json`
  - Command: `curl -I -sS --max-time 10 https://api.ubl.agency/mcp/rpc`
  - Command: `curl -I -sS --max-time 10 https://api.ubl.agency/mcp/sse`
  - Command: `curl -I -sS --max-time 10 https://logline.world/healthz`
  - Evidence: DNS resolution failed for all tested public hosts (2026-02-23T14:44:58Z); MCP content types could not be validated from this environment.
  - Update (2026-02-23T14:50:19Z): repeated public endpoint checks; DNS resolution still failed for all tested public hosts.
  - Update (2026-02-23T20:45:20Z): repeated public endpoint checks; DNS resolution still failed for all tested public hosts.
  - Update (2026-02-23T20:50:15Z): repeated public endpoint checks; DNS resolution still failed for ubl.agency, api.ubl.agency, logline.world.
  - Update (2026-02-24T02:44:47Z): repeated public endpoint checks; DNS resolution still failed for ubl.agency, api.ubl.agency, logline.world.
  - Update (2026-02-24T02:50:22Z): repeated public endpoint checks; DNS resolution still failed (curl: could not resolve host for api.ubl.agency).
  - Manual steps (operator):
    - From a network with external DNS, confirm:
      - `curl -I -sS https://api.ubl.agency/healthz`
      - `curl -I -sS https://api.ubl.agency/mcp/manifest`
      - `curl -I -sS https://api.ubl.agency/.well-known/webmcp.json`
      - `curl -I -sS https://api.ubl.agency/mcp/rpc` (expect `content-type: text/event-stream`)
