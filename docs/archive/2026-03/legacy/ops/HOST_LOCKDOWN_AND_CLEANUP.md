# Host Lockdown and Cleanup

**Status**: active  
**Owner**: Ops + Security  
**Last reviewed**: 2026-02-21

## Objective

Run UBL gate/tunnel continuously while making state access intentionally difficult for day-to-day operator accounts.

## Model

- Service identity: dedicated no-login user (default `ubl`).
- State directory ownership: service user only.
- Operator account: maintenance commands only (optional sudoers policy).
- Break-glass admin: separate administrative identity.

## Scripts

- `scripts/ubl_ops.sh` (official command router)
- `scripts/host_lockdown.sh`
- `scripts/workzone_cleanup.sh`
- `scripts/forever_bootstrap.sh`

## Official 3 Commands

1. `bootstrap-core`
2. `host-lockdown`
3. `ops-maintenance`

All three are exposed via `scripts/ubl_ops.sh` and `Makefile` targets.

## Remote Execution Pattern

1. Connect as admin on host.
2. Configure env (`ops/forever_bootstrap.env`).
3. Run host lockdown.
4. Run bootstrap.
5. Run work-zone cleanup in dry-run, review report, then apply.

## Commands

```bash
# 1) host lockdown (idempotent)
sudo bash scripts/ubl_ops.sh host-lockdown --env ops/forever_bootstrap.env

# 2) bootstrap
bash scripts/ubl_ops.sh bootstrap-core --env ops/forever_bootstrap.env

# 3) cleanup rehearsal
bash scripts/ubl_ops.sh ops-maintenance --env ops/forever_bootstrap.env --dry-run

# 4) cleanup apply
bash scripts/ubl_ops.sh ops-maintenance --env ops/forever_bootstrap.env
```

## Key Env Parameters

- Service user model:
  - `UBL_SERVICE_USER`
  - `UBL_SERVICE_GROUP`
  - `UBL_SERVICE_HOME`
  - `UBL_STATE_DIR`
  - `UBL_LOG_DIR`
- Operator/break-glass:
  - `UBL_OPERATOR_USER`
  - `UBL_CREATE_OPERATOR`
  - `UBL_ENABLE_SUDOERS_POLICY`
  - `UBL_BREAKGLASS_ADMIN`
  - `UBL_FAIL_ON_OPERATOR_BROAD_SUDO` (default `true`; blocks if operator has broad sudo grants)
- Cleanup:
  - `UBL_TMP_GLOBS`
  - `UBL_ALLOW_NON_TMP_GLOBS` (default `false`; non-`/tmp` globs are blocked)
  - `UBL_DOWNLOAD_RETENTION_DAYS`
  - `UBL_CLEAN_CARGO_CACHE`

## Safety Guarantees

- `workzone_cleanup.sh` never deletes genesis identity artifacts or runtime db paths.
- Cleanup emits a signed-by-hash JSON report under `${UBL_BASE_DIR}/state/maintenance/`.
- `host_lockdown.sh` is idempotent and can be re-run after host drift.
- `host_lockdown.sh` audits operator broad sudo exposure (`sudo/wheel/admin` group and broad sudoers rules).
