# Key Rotation Plan

**Status**: active  
**Owner**: Security  
**Last reviewed**: 2026-02-21

## Objective

Define deterministic and auditable rotation of `SIGNING_KEY_HEX` without breaking chain trust.

## Rotation Trigger Conditions

- key compromise suspected or confirmed
- planned cryptoperiod expiry
- compliance/security governance requirement

## Procedure

1. Freeze writes briefly (maintenance window) to avoid ambiguous overlap.
2. Generate new `SIGNING_KEY_HEX` and new `UBL_STAGE_SECRET` on LAB 512.
3. Derive and store new public key material.
4. Emit a **key rotation receipt** linking old identity to new identity.
5. Update runtime env and restart gate with controlled cutover.
6. Publish updated trust anchor hash externally.

## Key Rotation Receipt (minimum fields)

- `@type`: `ubl/key-rotation`
- `old_kid`
- `new_kid`
- `old_genesis_pubkey_sha256`
- `new_genesis_pubkey_sha256`
- `rotation_reason`
- `rotated_at` (UTC)
- signatures proving control continuity

## Revocation Signal

Revocation state for old key must be published in two places:

- local canonical record in bootstrap/security artifacts
- external witness publication (public, timestamped)

## Recovery / Rollback

- If cutover fails before any new receipt is emitted, rollback to previous runtime env.
- If new receipts already emitted, do not rollback silently; emit explicit corrective receipt.

## Audit Evidence Required

- updated key birth/provenance artifacts
- old/new public key hashes
- rotation receipt CID + JSON
- external witness reference
