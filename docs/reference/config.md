# Configuração oficial do UBL Gate

Fonte única de verdade: `crates/ubl_config/src/lib.rs`.

## Blocos de configuração

- **bind**: endereço/porta de escuta HTTP do Gate.
- **data_dir**: diretório base para persistência local.
- **storage**: backend e DSNs de persistência.
- **obs**: observabilidade (logs/tracing/métricas).

## ENVs e defaults (derivados do código)

- `UBL_GATE_BIND` (default: `0.0.0.0:4000`)
- `UBL_DATA_DIR` (default: `./data`)
- `UBL_STORE_BACKEND`
- `UBL_STORE_DSN`
- `UBL_IDEMPOTENCY_DSN`
- `UBL_OUTBOX_DSN`
- `UBL_OUTBOX_WORKERS`
- `UBL_OUTBOX_ENDPOINT`
- `UBL_EVENTSTORE_ENABLED`
- `UBL_EVENTSTORE_PATH`
- `RUST_LOG`

## Referências cruzadas

- Consumo no serviço: `services/ubl_gate/src/main.rs` e `services/ubl_gate/src/lib.rs`.
- Operação: `ops/gate/README.md`.
