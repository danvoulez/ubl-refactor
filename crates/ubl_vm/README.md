# ubl_vm

Deterministic VM for UBL transition execution.

## What is in this crate

- VM executor and fuel accounting in `src/exec.rs`
- TLV opcode encoding/decoding in `src/tlv.rs` and `src/opcode.rs`
- Canon helpers in `src/canon.rs`
- Runtime providers in `src/providers/`

## Tests and fixtures

- Law and property tests in `tests/`
- Canon/receipt vectors and regression tests in `tests/rho_contract_vectors.rs`, `tests/laws.rs`, and `tests/prop_vm.rs`

## Usage

This crate is consumed by `ubl_runtime` in the TR stage and by `ubl_cli` silicon compile/disasm flows.
