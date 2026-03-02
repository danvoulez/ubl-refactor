# WASM Conformance Vectors

This folder contains JSON conformance vectors used by the runtime WASM pipeline tests.

## What these vectors are

- **Positive vectors**: inputs expected to be accepted by the runtime profile checks.
- **Negative vectors**: inputs expected to be rejected with the expected decision/code pair.

These files are **test data** used to verify deterministic behavior and policy enforcement. They are not credentials and do not contain secrets.

## Folder structure

```text
docs/wasm/conformance/
└── vectors/
    └── v1/
        ├── positive/
        └── negative/
```

## Running tests that consume these vectors

From the repository root:

```bash
cargo test -p ubl_runtime stage_runtime_executes_wasm_conformance_vectors -- --nocapture
```

The test loader resolves vectors from:

1. `docs/wasm/conformance/vectors/v1` (current location)
2. `docs/archive/2026-03/legacy/wasm/conformance/vectors/v1` (legacy fallback)

## Schema

If a `VECTOR_SCHEMA_V1.json` file is present in this directory, vectors should conform to it.
