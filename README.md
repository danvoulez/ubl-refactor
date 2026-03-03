# UBL-CORE

<!-- Badges / “tags” -->
[![CI](https://img.shields.io/github/actions/workflow/status/LogLine-Foundation/ubl-refactor/ci.yml?branch=main&label=CI&logo=github)](https://github.com/LogLine-Foundation/ubl-refactor/actions)
[![Conformance](https://img.shields.io/github/actions/workflow/status/LogLine-Foundation/ubl-refactor/conformance.yml?branch=main&label=Conformance&logo=wasmer)](https://github.com/LogLine-Foundation/ubl-refactor/actions)
[![License](https://img.shields.io/github/license/LogLine-Foundation/ubl-refactor?label=License)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.90.x-000000?logo=rust)](./rust-toolchain.toml)
[![Release](https://img.shields.io/github/v/release/LogLine-Foundation/ubl-refactor?label=Release)](https://github.com/LogLine-Foundation/ubl-refactor/releases)
[![Issues](https://img.shields.io/github/issues/LogLine-Foundation/ubl-refactor?label=Issues)](https://github.com/LogLine-Foundation/ubl-refactor/issues)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](./docs/contributing.md)
[![Security](https://img.shields.io/badge/Security-security%40logline.foundation-blue)](./SECURITY.md)

UBL-CORE is the Rust workspace behind the LogLine ecosystem: a **deterministic core** plus a **production-friendly Gate service** for generating and verifying canonical outputs (receipts, verification artifacts, etc.).

If you’ve ever shipped something that *worked in staging but behaved differently in prod*, UBL-CORE is built to fight that class of problem: **stable behavior, explicit config, repeatable results.**

- Deterministic pipeline: `KNOCK → WA → CHECK → TR → WF`
- Gate runs as a service (Docker/systemd/compose-friendly)
- Config is centralized (no mystery env vars hiding in random files)
- WASM conformance vectors included + runnable in CI

**Start here:** [`START-HERE.md`](./START-HERE.md)  
**Docs hub:** [`docs/index.md`](./docs/index.md)

---

## Why UBL-CORE?

A few plain-English reasons people use it:

- **Determinism you can trust:** the same input produces the same canonical output — across machines and environments.
- **Config you can reason about:** behavior is driven by a single config contract (not “env var archaeology”).
- **A boring, reliable service:** the Gate is designed to be deployable and operable without surprises.
- **Tested like it matters:** conformance vectors and runtime tests help prevent drift and regressions.

---

## Quickstart

### Requirements
- Rust is pinned via `rust-toolchain.toml` (Rust **1.90.x** recommended)

### Build & test
```bash
make quality-gate
cargo test --workspace
````

### Run the Gate (service)

```
make gate
curl -fsS http://127.0.0.1:4000/healthz
```

* * *

What’s inside?
--------------

*   `crates/` — core libraries (canon, runtime, receipts, config, etc.)
*   `services/ubl_gate/` — the Gate service (thin `main`, real wiring in the library)
*   `ops/` — Docker/compose/systemd templates + runbooks
*   `docs/` — the official docs tree (start at `docs/index.md`)

* * *

The Gate service
----------------

The Gate is meant to be boring (the good kind):

1.  load one **AppConfig**
2.  validate it
3.  start the router + background workers
4.  expose health endpoints for ops

**Ops docs:**

*   Gate overview: `ops/gate/README.md`
*   Incident runbook: `ops/runbooks/gate.md`

* * *

Configuration (the “source of truth”)
-------------------------------------

Everything the service cares about is defined in the config contract (in `ubl_config`). That includes Gate settings, storage, observability, URLs, limits, write policy, build info, LLM settings, and crypto mode (with redaction for secrets).

*   Config reference: `docs/reference/config.md`

* * *

Determinism & canonical behavior
--------------------------------

UBL-CORE is built around **repeatability**:

*   the pipeline doesn’t rely on ambient environment drift
*   cryptographic mode is parsed once and propagated explicitly
*   tests cover determinism and cross-mode rejection

If you want the “mental model”, read `START-HERE.md` and follow the links from `docs/index.md`.

* * *

WASM conformance
----------------

This repo includes public conformance vectors used by runtime tests:

*   Vectors live at: `docs/wasm/conformance/vectors/v1/`
*   Overview: `docs/wasm/conformance/README.md`

* * *

Releases
--------

*   Release process: `docs/release/process.md`
*   Pre-tag checklist: `docs/release/checklist.md`
*   Changelog: `CHANGELOG.md`

* * *

Contributing
------------

Small PRs are welcome: clearer contracts, better tests, less drift, better docs.

*   Contributing: `docs/contributing.md`
*   Testing: `docs/contributing/testing.md`

* * *

Security
--------

Please **don’t** report vulnerabilities in public issues.

*   Security policy: `SECURITY.md`
*   Contact: **security@logline.foundation**
*   Threat model: `docs/security/threat-model.md`

* * *

Community & governance
----------------------

*   Maintainers: `docs/MAINTAINERS.md`
*   Governance: `docs/governance.md`
*   Support: `SUPPORT.md`

* * *

License & trademarks
--------------------

*   License: `LICENSE` (plus `NOTICE` / `COPYRIGHT` if present)
*   Trademark policy: `TRADEMARK_POLICY.md`
*   Commercial licensing: `COMMERCIAL-LICENSING.md`

* * *

Tags (for humans)
-----------------

deterministic · canonical · receipts · verification · rust · crypto · wasm · conformance · runtime · pipeline · auditability · reproducibility · security · infra · service · observability · SLO · operations

````

### “Tons of tags” for GitHub Topics (paste into repo settings)
Here’s a big, clean set you can paste into **Settings → Topics**:

```text
rust, deterministic, reproducible-builds, canonicalization, cryptography, receipts, verification, attestations, auditability, runtime, pipeline, wasm, conformance, security, infra, microservice, observability, slo, devops, systemd, docker, supply-chain, integrity, provenance
````
