# Rollout Automation

This document is the operational automation reference for release rollout.

## Canonical operators and runbooks

- Gate operations: `ops/gate/README.md`
- Production rollout policy: `docs/release/rollout.md`
- Release checklist: `docs/release/checklist.md`

## Required automation outputs

- Deterministic contract report: `artifacts/contract/latest.json`
- Deterministic conformance report: `artifacts/conformance/latest.json`
- Signed receipt evidence from rollout actions
