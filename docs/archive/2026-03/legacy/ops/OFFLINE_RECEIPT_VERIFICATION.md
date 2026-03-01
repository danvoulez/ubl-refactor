# Offline Receipt Verification

**Status**: planned (interface specified)  
**Owner**: Core Runtime + CLI  
**Last reviewed**: 2026-02-21

## Goal

Verify receipts without requiring a running gate/runtime.

## Required Inputs

- receipt JSON file
- public key (PEM or DID key material)
- canonical bytes specification version (`NRF-1` + domain rules)

## Current State

- `ublx` currently has `explain`, `verify` (chip), and related commands.
- A dedicated `verify-receipt` subcommand is not yet present in `ublx`.

## Proposed CLI Interface

```bash
ublx verify-receipt \
  --receipt /path/to/receipt.json \
  --pubkey /path/to/genesis_signer.pub.pem
```

Expected behavior:

- canonicalize receipt payload deterministically
- verify Ed25519 signature domain separation
- return machine-readable status (`valid`/`invalid`) and reason
- non-zero exit code on verification failure

## Acceptance Criteria

- can verify bootstrap receipt #0 offline from exported artifacts
- no network/runtime dependency
- deterministic result across machines/toolchains
