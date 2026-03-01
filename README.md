# UBL CORE

Open-source deterministic runtime for chip processing, cryptographic receipts, policy enforcement, and operational rollout controls.

## Scope

`UBL-CORE` is the OSS foundation:

- deterministic gate pipeline (`KNOCK -> WA -> CHECK -> TR -> WF`)
- runtime and canon primitives
- receipts and attestation
- CLI, MCP gateway, stores, and core connectors

Product shells stay outside this repo and currently live at:
- [danvoulez/UBL-SHELLS](https://github.com/danvoulez/UBL-SHELLS)

## Quick Start

```bash
cargo build --workspace
cargo test --workspace
cargo run -p ubl_gate
```

Gate default: `http://localhost:4000`

## Quality Workflow

```bash
# Contract-first checks (black-box invariants)
make contract

# Full compatibility contract (conformance vectors + reports)
make conformance

# Full local gate before PR
make quality-gate
```

Process references:
- `TEST_STRATEGY.md`
- `QUALITY_GATE.md`

## Primary Endpoints

- `POST /v1/chips`
- `GET /v1/chips/:cid`
- `GET /v1/chips/:cid/verify`
- `GET /v1/receipts/:cid/trace`
- `GET /v1/receipts/:cid/narrate`
- `GET /v1/receipts/:cid/url`
- `GET /v1/runtime/attestation`
- `GET /metrics`
- `GET /openapi.json`
- `POST /mcp/rpc`

## Documentation

- Primary entry point (LLM-first): `docs/START-HERE-LLM-FIRST.md`
- Secondary index: `docs/INDEX.md`
- Canonical runtime entrypoint: `START-HERE.md`
- Architecture: `ARCHITECTURE.md`
- Security policy: `SECURITY.md`
- Governance: `GOVERNANCE.md`
- RFC process: `RFC_PROCESS.md`
- Versioning policy: `VERSIONING.md`
- Compatibility policy: `COMPATIBILITY.md`
- OSS scope/boundary: `docs/oss/OPEN_SOURCE_SCOPE.md`
- Trust charter: `docs/oss/LOGLINE_TRUST_CHARTER.md`
- Contributing: `CONTRIBUTING.md`
- Code of conduct: `CODE_OF_CONDUCT.md`
- Support policy: `SUPPORT.md`
- Trademark policy: `TRADEMARK_POLICY.md`
- Commercial model: `COMMERCIAL-LICENSING.md`
- Docs attestation: `docs/security/DOC_ATTESTATION.md`
- Forever host bootstrap runbook: `docs/ops/FOREVER_BOOTSTRAP.md`
- Gitea + S3 signed source flow: `docs/ops/GITEA_SOURCE_FLOW.md`
- MCP runtime validation: `docs/ops/MCP_RUNTIME_VALIDATION.md`
- Cloudflare edge baseline: `docs/ops/CLOUDFLARE_EDGE_BASELINE.md`
- Host lockdown and cleanup: `docs/ops/HOST_LOCKDOWN_AND_CLEANUP.md`
- Key rotation plan: `docs/key_rotation_plan.md`
- Offline receipt verification: `docs/ops/OFFLINE_RECEIPT_VERIFICATION.md`

## Release Model

- Pushing a tag `v*` creates a candidate release automatically.
- Manual promotion to official/latest is done via workflow `Release From Tag`.

## License

Licensed under Apache-2.0. See `LICENSE`.
See also `NOTICE` and `COPYRIGHT`.

## Security Notes

- Production signature path is Ed25519 (receipt/runtime attestations).
- PQ dual-sign (`ML-DSA3`) is feature-gated as a rollout stub (`ubl_kms/pq_mldsa3`): wire/API shape exists and PQ signature currently returns `None` until backend integration is completed.
