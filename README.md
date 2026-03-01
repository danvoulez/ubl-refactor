# UBL-CORE

UBL-CORE é o workspace Rust oficial para runtime determinístico, recibos criptográficos e operação do serviço **UBL Gate**.

## Visão em 30s

- Runtime/canon/config ficam em crates (`ubl_runtime`, `ubl_config`, etc.).
- O serviço HTTP é o `ubl_gate`.
- Compat legada existe via shims (`rb_vm`, `ubl_ai_nrf1`) apenas para integração de suíte.

## Links principais

- [Start Here](START-HERE.md)
- [Docs Index oficial](docs/index.md)
- [Ops do Gate](ops/gate/README.md)

## Quick run

```bash
make gate
curl -fsS http://127.0.0.1:4000/healthz
```

## Toolchain

- Rust **1.90.0** (pinned em `rust-toolchain.toml`)
- Componentes obrigatórios: `rustfmt`, `clippy`
