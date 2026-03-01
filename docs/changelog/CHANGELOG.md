# Changelog

**Status**: active
**Owner**: Repo Maintainer
**Last reviewed**: 2026-02-21

## 2026-02-21

- OSS boundary/docs hardening:
  - added `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md`, `LICENSE`
  - added `docs/oss/OPEN_SOURCE_SCOPE.md`
  - aligned shell repository contract to `https://github.com/danvoulez/UBL-SHELLS`
- Release and integration automation:
  - added tag-driven release workflow: `.github/workflows/release-from-tag.yml`
  - added core-to-shell notification workflow: `.github/workflows/notify-shells.yml`
- CI stability hardening:
  - fixed formatting/clippy blockers across runtime/gate/tests so `cargo fmt --check` and `cargo clippy -D warnings` pass locally
- Dependency hygiene:
  - removed unused `redis` dependency from `UBL-CORE` manifests to eliminate Rust future-incompatibility warning from `redis v0.25.4`
  - recorded decision in `docs/adr/0004-REDIS-DEPENDENCY-HARDENING.md`

## 2026-02-20

- Promoted root-level governance and security entry documents:
  - `SECURITY.md` (canonical security/trust model)
  - `GOVERNANCE.md` (project/process governance)
- Removed manual markdown reference pages to avoid drift:
  - `docs/reference/API.md`
  - `docs/reference/CONFIG.md`
  - `docs/reference/ERRORS.md`
  - `docs/reference/NUMERICS.md`
- Switched reference indexing to official code-exported sources:
  - `/openapi.json` + `crates/ubl_runtime/src/manifest.rs`
  - `crates/ubl_runtime/src/error_response.rs`
  - `schemas/unc-1.schema.json`
  - `docs/reference/README.md` (source map)
- Kept compatibility pointers for previous location-based links:
  - `docs/security/CRYPTO_TRUST_MODEL.md` -> `SECURITY.md`
- Updated repository references (`README.md`, `docs/INDEX.md`, `TASKLIST.md`, `CERTIFIED_RUNTIME.md`).

## 2026-02-17

- Added formal documentation governance:
  - `docs/INDEX.md`
  - `docs/STANDARDS.md`
  - `docs/adr/*`
  - `docs/reference/API.md`
  - `docs/reference/CONFIG.md`
  - `docs/reference/ERRORS.md`
  - `SECURITY.md` (later promoted to root canonical location)
  - `docs/lifecycle/RELEASE_READINESS.md`
- Archived superseded strategy/checklist/tasklist docs into `docs/archive/2026-02/`.
- Updated canonical entry documents (`README.md`, `ARCHITECTURE.md`, `TASKLIST.md`).

## 2026-02-18

- Closed F4/H6 execution slice (property-testing expansion + typed parse boundary in pipeline critical paths).
- Hardened CI:
  - fixed `KNOCK` output contract handling in GitHub Actions
  - added UNC-1 strict smoke checks under CI (`REQUIRE_UNC1_NUMERIC=true`, `F64_IMPORT_MODE=reject`)
  - added optional `SIMKIT-STX` CI job (auto-skips when `simrunner/main.py` is absent)
- Security dependency upgrades:
  - `ring` -> `0.17.x`
  - `prometheus` -> `0.14.x` (moves protobuf chain to `3.7.2+`)
  - `reqwest` -> `0.12.x`
- `ubl_ledger` no longer exposes no-op public API:
  - default feature `ledger_mem`
  - opt-in feature `ledger_ndjson` append-only persistence backend
- Draft release notes added: `docs/changelog/V0_4_0_RC1.md`.
