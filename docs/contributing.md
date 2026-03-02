# Contributing Guide

## Development setup

- Rust toolchain pinned in `rust-toolchain.toml`.
- Install `rustfmt` and `clippy`.

## Local checks

Run before opening a PR:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
make quality-gate
```

## Pull requests

- Keep changes scoped and reviewable.
- Update docs when behavior/contracts change.
- Include test evidence and rationale.
