# UNC-1 Numeric Canon (Normative)

UNC-1 defines canonical numeric atoms for UBL.

## Rules

- Raw floating-point literals are not canonical payload data.
- Canonical numerics must use `@num` tagged objects validated by `schemas/unc-1.schema.json`.
- KAT vectors for interoperability are in `kats/unc1/unc1_kats.v1.json`.

## Canonical references

- Schema: `schemas/unc-1.schema.json`
- KATs: `kats/unc1/unc1_kats.v1.json`
- Runtime enforcement tests: `crates/ubl_runtime/src/knock.rs`
