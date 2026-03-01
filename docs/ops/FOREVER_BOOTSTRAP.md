# Forever Bootstrap Hardening List

**Status**: active  
**Owner**: Ops + Security  
**Last reviewed**: 2026-02-21

This runbook defines permanent ("forever") bootstrap controls for LAB 512.
Bootstrap is treated as a non-repeatable genesis event: artifacts not captured at genesis may be impossible to reconstruct with confidence later.

## Scope

- Script: `scripts/forever_bootstrap.sh`
- Env template: `ops/forever_bootstrap.env.example`

## 21 Hardening Controls

1. Machine birth certificate (immutable)
   - Artifact: `${UBL_BASE_DIR}/state/machine_birth.json`
   - Captures OS/kernel/arch/toolchain/host/MAC/disk serial (if available).
   - Rule: if artifact exists, script logs `already exists` and never overwrites.

2. Key birth certificate (immutable)
   - Artifact: `${UBL_BASE_DIR}/state/key_birth.json`
   - Captures UTC timestamp, OS fingerprint, release tag/commit, gate binary hash, genesis pubkey hash.
   - Rule: script refuses key regeneration when `key_birth.json` exists unless `UBL_ALLOW_KEY_BIRTH_OVERRIDE=true`.

3. Genesis public key export + trust anchor
   - Artifacts:
     - `${UBL_BASE_DIR}/state/genesis_signer.pub.pem`
     - `${UBL_BASE_DIR}/state/genesis_signer.pub.pem.sha256`
   - Script derives Ed25519 public key from `SIGNING_KEY_HEX` at genesis.
   - Operator requirement: publish the public key hash externally (release notes/README/issue).

4. Commit SHA provenance
   - Artifact: `${UBL_BASE_DIR}/state/release_provenance.json`
   - Script captures commit SHA from extracted tarball dir when available.
   - Rule: this SHA is canonical provenance for the genesis build.
   - Trust pinning:
     - `UBL_ATTEST_PUBKEY_SHA256` pins the attestation signer key (outside release assets).
     - `UBL_TARBALL_SHA256` pins the source tarball hash (outside release assets).
     - Unpinned mode requires explicit override via `UBL_ALLOW_UNPINNED_*`.

5. Binary reproducibility check (observability)
   - Optional env: `UBL_EXPECTED_GATE_SHA256`
   - Script records match/mismatch in `release_provenance.json`.
   - Rule: mismatch is logged for audit, not a bootstrap hard-fail.

6. Receipt #0 external witness
   - Artifact: `${UBL_BASE_DIR}/state/bootstrap/witness.json`
   - Contains `receipt0_sha256`, `genesis_pubkey_sha256`, `release_commit`, UTC timestamp.
   - Preferred automation: `UBL_EXTERNAL_WITNESS_BIN` (receives witness file path in `WITNESS_FILE` env and as argv[1]).
   - Legacy automation: `UBL_EXTERNAL_WITNESS_CMD` is supported only when `UBL_ALLOW_SHELL_WITNESS_CMD=true`.
   - Strict mode: `UBL_REQUIRE_EXTERNAL_WITNESS=true` fails if no external publication command succeeds.

7. Immediate post-bootstrap snapshot
   - Artifact root: `${UBL_BASE_DIR}/state/snapshots/<timestamp>/`
   - Includes `runtime.public.env` (redacted), SQLite, bootstrap artifacts, attestation files.
   - Optional secret inclusion: `UBL_SNAPSHOT_INCLUDE_RUNTIME_SECRETS=true` adds full `runtime.env` (default is `false`).
   - Rule: never overwrite snapshot path; operator must copy to encrypted off-site storage immediately.

8. SQLite backup from day 1
   - Script installs encrypted backup runner + cron and executes first backup during bootstrap.
   - Required envs:
     - `UBL_BACKUP_DEST`
     - `UBL_BACKUP_ENCRYPTION_PASSPHRASE_FILE`
     - optional hardening:
       - `UBL_BACKUP_ENCRYPTION_MODE=auto|openssl-gcm|openssl-cbc-hmac`
       - `UBL_BACKUP_PBKDF2_ITER`
       - `UBL_BACKUP_INCLUDE_RUNTIME_SECRETS=false` (default; backup includes `runtime.public.env` only)
   - Rule: bootstrap fails if backup setup/first encrypted backup fails.

9. PM2 startup registration
   - Script runs `pm2 startup` after `pm2 save`.
   - If root escalation is required, script logs manual command from PM2 output.

10. cloudflared service trade-off
    - Default: tunnel process managed by PM2 for simplicity.
    - Optional: `UBL_CLOUDFLARE_SERVICE_INSTALL=true` to install cloudflared system service.
    - Mandatory guard: `UBL_CLOUDFLARE_ACCESS_POLICY_CONFIRMED=true` is required when `UBL_CLOUDFLARE_ENABLE=true`.
    - Trade-off is explicitly documented in script behavior.

11. Bootstrap anti re-emit
    - Rule: if `${UBL_BASE_DIR}/state/bootstrap/receipt.json` exists, script does not POST bootstrap chip again.
    - Prevents duplicate/confusing genesis receipts on partial reruns.

12. Log rotation
    - Script installs/configures `pm2-logrotate` baseline:
      - daily rotation
      - retention 90
      - compression enabled

13. Edge rate limiting registry
    - Artifact: `${UBL_BASE_DIR}/state/cloudflare_rate_limit.json` (if provided)
    - Operator supplies `UBL_CLOUDFLARE_RATE_LIMIT_RULES` with rule IDs/names for:
      - `/v1/receipts`
      - `/v1/chips`

14. Database schema marker
    - Script computes schema marker as SHA-256 of ordered SQLite DDL.
    - Stored in `${UBL_BASE_DIR}/state/release_provenance.json` as `schema_version`.

15. Heartbeat receipts (chain alive)
    - Script installs heartbeat cron emitting lightweight receipts periodically.
    - Env controls:
      - `UBL_HEARTBEAT_ENABLE`
      - `UBL_HEARTBEAT_CRON`
      - `UBL_HEARTBEAT_WORLD`
      - `UBL_HEARTBEAT_ID_PREFIX`

16. Key rotation plan (documented now)
    - Artifact: `docs/key_rotation_plan.md`
    - Defines rotation procedure, linking receipt, and revocation signal location.

17. Offline receipt verification model
    - Artifact: `docs/ops/OFFLINE_RECEIPT_VERIFICATION.md`
    - Current state + required interface for fully offline verification are specified.

18. TLS trust model documentation
    - This deployment trusts TLS termination at Cloudflare edge.
    - If direct/off-Cloudflare access is introduced, separate PKI/cert strategy is mandatory.

19. Second authorized human continuity
    - Requirement: explicit ownership/continuity decision must be recorded as policy.
    - If absent, risk acceptance must be documented and reviewed periodically.
    - Artifact: `ops/continuity_policy.md`

20. Postmortem template ready before incidents
    - Artifact: `ops/postmortem_template.md`

21. Ecosystem file single-write hygiene
    - Script writes PM2 ecosystem file once using conditional app list assembly.
    - Avoids double-write divergence.

## Operational Addendum: Freeze Marker

- Artifact: `${UBL_BASE_DIR}/state/freeze_manifest.json`
- Goal: establish a canonical "frozen state" reference after cleanup/hardening.
- Contents:
  - genesis layer directory-tree hash (`genesis_layer.tree_sha256`) plus per-file hashes
  - running services list
  - open ports list
  - installed packages with versions
  - disk usage snapshot
  - UTC timestamp
- Idempotency:
  - default behavior is immutable (`UBL_FREEZE_MANIFEST_OVERWRITE=false`)
  - overwrite is explicit and intentional (`UBL_FREEZE_MANIFEST_OVERWRITE=true`)

## Operational Addendum: Genesis Symbol

- Artifact: `${UBL_BASE_DIR}/state/genesis_symbol.json`
- Goal: keep a single immutable symbol document with the permanent anchor tuple:
  - `genesis_pubkey_sha256`
  - `receipt0_sha256`
  - `receipt_cid`
- References linked inside the artifact:
  - trust anchor PEM path
  - receipt path
  - witness path
  - key birth path
  - release provenance path
- Idempotency:
  - default immutable (`UBL_SYMBOL_OVERWRITE=false`)
  - overwrite only by explicit operator action (`UBL_SYMBOL_OVERWRITE=true`)

## Bootstrap Definition of Done (DoD)

Bootstrap is complete only when all conditions below are true.

### Local artifacts

- `${UBL_BASE_DIR}/state/machine_birth.json`
- `${UBL_BASE_DIR}/state/key_birth.json`
- `${UBL_BASE_DIR}/state/genesis_signer.pub.pem`
- `${UBL_BASE_DIR}/state/genesis_signer.pub.pem.sha256`
- `${UBL_BASE_DIR}/state/release_provenance.json` with `release_commit` and `schema_version`
- `${UBL_BASE_DIR}/state/bootstrap/receipt.json` (receipt #0)
- `${UBL_BASE_DIR}/state/snapshots/<timestamp>/...` snapshot present
- `${UBL_BASE_DIR}/state/freeze_manifest.json`
- `${UBL_BASE_DIR}/state/genesis_symbol.json`

### External evidence

- witness for receipt #0 published externally (via `UBL_EXTERNAL_WITNESS_CMD` or manual publication)
- genesis public key hash published as trust anchor

### Operations

- encrypted backup cron installed and first backup artifact created
- PM2 startup registered or manual root command logged
- PM2 logrotate configured

## Run

```bash
cp ops/forever_bootstrap.env.example ops/forever_bootstrap.env
# edit ops/forever_bootstrap.env
# optional but recommended before bootstrap (root/admin):
# sudo bash scripts/host_lockdown.sh --env ops/forever_bootstrap.env
bash scripts/forever_bootstrap.sh --env ops/forever_bootstrap.env
```

Dry-run planning:

```bash
bash scripts/forever_bootstrap.sh --env ops/forever_bootstrap.env --dry-run
```
