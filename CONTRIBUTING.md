# Contributing to UBL-CORE

Thanks for contributing. `UBL-CORE` is the open-source foundation of the UBL runtime stack.

## Before You Start

- Read `README.md` and `docs/INDEX.md`.
- For behavior/security changes, review `ARCHITECTURE.md` and `SECURITY.md`.
- For repo boundaries (core vs shells), read `docs/oss/OPEN_SOURCE_SCOPE.md`.
- For high-impact contract changes, read `RFC_PROCESS.md` and open RFC first.

## Local Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If your change affects runtime behavior, also test the gate:

```bash
cargo run -p ubl_gate
```

## Pull Request Rules

1. Keep PRs focused (single concern when possible).
2. Update docs in the same PR when behavior/interfaces change.
3. Add or update tests for changed behavior.
4. Avoid vendoring/copying code from product-shell repositories into core.
5. Use clear commit/PR descriptions that explain intent and impact.
6. If change affects protocol/API/CLI/MCP compatibility, link the accepted RFC.

## Security Issues

Do not open public issues for vulnerabilities.
Follow `SECURITY.md` for private/coordinated disclosure.
