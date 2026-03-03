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

1. `docs/wasm/conformance/vectors/v1`

## Schema

Vectors **must** conform to `VECTOR_SCHEMA_V1.json` in this directory. Validate with `bash scripts/wasm_conformance.sh --mode contract`.
