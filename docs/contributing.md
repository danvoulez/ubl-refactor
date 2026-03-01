# Contributing

## Setup

- Rust 1.90.0 (`rust-toolchain.toml`)
- `rustfmt` e `clippy`

## Fluxo local mínimo

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
make quality-gate
```

## Estilo de commits e branches

- Commits pequenos e objetivos.
- Mensagens no formato `<tipo>: <resumo>`.
- Branch com escopo claro de mudança.

## Regras de documentação

- Atualize `docs/index.md` quando adicionar/remover doc oficial.
- Evite duplicação de contrato já definido no código.
