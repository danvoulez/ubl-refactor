# WASM Execution Charter

**Status**: Draft
**Version**: `v1`
**Date**: 2026-02-23

## Purpose

Define the non-negotiable constraints for UBL WASM execution hardening.

## Scope

- ABI contract
- capability isolation
- deterministic profile
- integrity/attestation gate
- resource guards
- receipt claim binding
- error taxonomy

## Success Criteria

- deterministic and auditable behavior
- fail-closed security posture
- conformance gate in CI
- release-ready evidence bundle

## Non-Goals

- replacing the full native runtime with WASM-only execution
- adding permissive host capabilities without policy model

## Governance

Any spec change requires:

1. explicit versioned edit in this spec pack
2. new/updated conformance vectors
3. CI evidence in conformance report
