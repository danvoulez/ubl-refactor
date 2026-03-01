# Docs Attestation

**Status**: active  
**Owner**: Security  
**Last reviewed**: 2026-02-21

## Goal

Make governance/security/standard documents tamper-evident with deterministic manifest + cryptographic signature.

## What Is Signed

`scripts/docs_attest.sh build-manifest` creates a deterministic manifest with:

- canonical file list
- per-file SHA-256
- deterministic tree hash
- git metadata (commit/tag)

`scripts/docs_attest.sh sign` signs the manifest with Ed25519 and emits an attestation bundle.

## Local Workflow

1. Generate keypair (encrypted if `DOCS_ATTEST_KEY_PASSPHRASE` is set):

```bash
scripts/docs_attest.sh init-key \
  --key-out ~/.ubl-core/keys/docs_attest_ed25519.pem \
  --pub-out ~/.ubl-core/keys/docs_attest_ed25519.pub.pem
```

2. Build manifest:

```bash
scripts/docs_attest.sh build-manifest --out ./release-artifacts/docs/manifest.json
```

3. Sign manifest:

```bash
scripts/docs_attest.sh sign \
  --manifest ./release-artifacts/docs/manifest.json \
  --key ~/.ubl-core/keys/docs_attest_ed25519.pem \
  --pub ~/.ubl-core/keys/docs_attest_ed25519.pub.pem \
  --out ./release-artifacts/docs/attestation.json
```

4. Verify:

```bash
scripts/docs_attest.sh verify \
  --manifest ./release-artifacts/docs/manifest.json \
  --attestation ./release-artifacts/docs/attestation.json
```

## Biometric Protection (Face ID / Touch ID)

Practical rule: keep one stable signing key, unlock it with biometric-protected secret storage.

- Avoid "new wallet every signature" for release identity; rotating key each run breaks trust continuity.
- Prefer persistent key + biometric-gated unlock.

On Apple devices:

- macOS generally uses Touch ID (not Face ID) for local keychain prompts.
- You can store the key passphrase in Keychain and configure access control to require biometric/user presence.
- For stronger guarantees, use hardware-backed keys (for example FIDO2/PIV token with touch presence) and keep private key non-exportable.

Example (store passphrase in Keychain):

```bash
security add-generic-password -a "$USER" -s "ubl-docs-attest-passphrase" -w "<passphrase>" -U
```

Example (retrieve for signing session):

```bash
export DOCS_ATTEST_KEY_PASSPHRASE="$(security find-generic-password -a "$USER" -s "ubl-docs-attest-passphrase" -w)"
```

## Release Integration

`release-from-tag` workflow publishes docs attestation artifacts with release assets:

- `manifest.json`
- `attestation.json`
- `attestation.sig`
- `attestation.sig.b64`
- public key used for verification

Required repository secrets for signed official release:

- `DOCS_ATTEST_PRIVATE_KEY_PEM_B64`: base64 of private key PEM
- `DOCS_ATTEST_PUBLIC_KEY_PEM_B64` (optional): base64 of public key PEM
- `DOCS_ATTEST_KEY_PASSPHRASE` (optional): passphrase if private key is encrypted

Example with GitHub CLI:

```bash
gh secret set DOCS_ATTEST_PRIVATE_KEY_PEM_B64 < <(base64 < ~/.ubl-core/keys/docs_attest_ed25519.pem | tr -d '\n')
gh secret set DOCS_ATTEST_PUBLIC_KEY_PEM_B64 < <(base64 < ~/.ubl-core/keys/docs_attest_ed25519.pub.pem | tr -d '\n')
gh secret set DOCS_ATTEST_KEY_PASSPHRASE < <(security find-generic-password -a "$USER" -s "ubl-docs-attest-passphrase" -w)
```

## Security Notes

- Do not commit private keys.
- Commit or publish trusted public keys.
- Rotate keys with overlap period: old key still verifies historical releases while new key signs new tags.
