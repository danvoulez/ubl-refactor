# Cloudflare Tunnel Go-Live Checklist

**Status**: active  
**Owner**: Ops + Security  
**Last reviewed**: 2026-02-21

Use this checklist before enabling public exposure for UBL Gate.

## 1) Identity and least privilege

- Use API Tokens, never Global API Key.
- Minimum scopes:
  - Account: `Cloudflare Tunnel Edit`
  - Zone: `DNS Edit` (only for required zone)
- Store token in secrets manager, not in shell history.

## 2) Register tunnel and app before exposure

- Create/confirm tunnel in Cloudflare Zero Trust.
- Create Access application and policy first.
- Decide auth mode (`email`, IdP, service token) before DNS cutover.
- Enable token validation at origin (`Protect with Access`) or implement manual JWT validation.
- In bootstrap env, only set:
  - `UBL_CLOUDFLARE_ENABLE=true`
  - `UBL_CLOUDFLARE_ACCESS_POLICY_CONFIRMED=true`
  after Access policy is active.

## 3) Ingress safety

- Ensure tunnel ingress has explicit terminal catch-all:
  - `service: http_status:404`
- Ensure only intended hostname is routed.
- Avoid wildcard routes until policy is validated.

## 4) DNS safety

- Tunnel hostname record must be proxied `CNAME` to:
  - `<TUNNEL_ID>.cfargotunnel.com`
- For apex hostname (e.g., `logline.world`), rely on Cloudflare flattening.
- Keep TTL/propagation expectations documented for cutover window.

## 5) Service resilience

- Run `cloudflared` as system service when possible.
- If using PM2, ensure startup registration was completed.
- Verify tunnel auto-recovers on reboot before production traffic.

## 6) Edge protections

- Configure rate-limits for:
  - `/v1/chips`
  - `/v1/receipts`
- Record rule IDs/names in:
  - `${UBL_BASE_DIR}/state/cloudflare_rate_limit.json`

## 7) Verification before go-live

- Local health:
  - `curl -fsS http://127.0.0.1:4000/healthz`
- Public health (through tunnel):
  - `curl -fsS https://<domain>/healthz`
- Access policy check:
  - anonymous request must be denied by Access policy.

## 8) Secret hygiene and rotation

- Treat `CLOUDFLARE_TUNNEL_TOKEN` as high-sensitivity credential.
- Rotate token on staff/device changes.
- Keep an explicit rotation playbook and owner.

## 9) Break-glass

- Keep temporary disable path documented:
  1. pause public DNS route, or
  2. stop tunnel process, or
  3. tighten Access policy to deny all.
- Record every break-glass action in incident notes.
