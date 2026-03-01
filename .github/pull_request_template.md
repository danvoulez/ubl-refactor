## Summary

Describe the behavior change in one paragraph.

## Contract Impact

- [ ] No external behavior change
- [ ] External behavior changed and contract tests/vectors were updated first

If behavior changed, list the contract files touched:

- `...`

## RED-FIRST EVIDENCE (required for behavior changes and bug fixes)

Paste command(s) and output snippet showing failing test before implementation.

```bash
# command
```

```text
# failing output snippet
```

## GREEN EVIDENCE

Paste command(s) and output snippet showing passing state after implementation.

```bash
# command
```

```text
# passing output snippet
```

## Conformance

- [ ] `scripts/conformance_suite.sh` was run (or CI conformance passed)
- [ ] Any changed contract surface is covered by conformance vectors/rules

Link artifact/CI run:

- `...`

## Checklist

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `scripts/contract_suite.sh`
- [ ] `cargo test --workspace --lib`
- [ ] `scripts/conformance_suite.sh`
- [ ] `TEST_STRATEGY.md` and `QUALITY_GATE.md` respected
