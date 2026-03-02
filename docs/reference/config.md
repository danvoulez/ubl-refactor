# Configuração oficial do UBL Gate

Fonte única de verdade: `crates/ubl_config/src/lib.rs`.

## Blocos de configuração (`AppConfig`)

- `gate`: bind/data_dir.
- `storage`: backend, DSNs, outbox e eventstore.
- `obs`: nível de logs/tracing.
- `urls`: URLs públicas/manifest.
- `limits`: limites de canon e MCP.
- `write`: política de escrita pública/autenticada.
- `build`: metadados de build publicados pelo Gate.
- `llm`: configuração do backend LLM (OpenAI/local).
- `crypto`: modo criptográfico para compatibilidade/métricas.

## ENVs e defaults/fallbacks

### Gate

- `UBL_GATE_BIND` → default `0.0.0.0:4000`.
- `UBL_DATA_DIR` → default `./data`.

### Storage

- `UBL_STORE_BACKEND` → default `memory`.
- `UBL_STORE_DSN` → opcional (trim; vazio = `None`).
- `UBL_IDEMPOTENCY_DSN` → opcional (trim; vazio = `None`).
- `UBL_OUTBOX_DSN` → opcional (trim; vazio = `None`).
- `UBL_OUTBOX_WORKERS` → default `1`, mínimo `1`.
- `UBL_OUTBOX_ENDPOINT` → opcional (trim; vazio = `None`).
- `UBL_EVENTSTORE_ENABLED`:
  - ENV ausente ⇒ `true`.
  - ENV presente ⇒ `true` somente para `1|true|TRUE|yes|on`.
- `UBL_EVENTSTORE_PATH` → default `./data/events`.

DSN efetivo para backend sqlite segue prioridade:

1. `UBL_STORE_DSN`
2. `UBL_IDEMPOTENCY_DSN`
3. `UBL_OUTBOX_DSN`
4. fallback `file:./data/ubl.db?mode=rwc&_journal_mode=WAL`

### Observabilidade

- `RUST_LOG` → default `info,ubl_runtime=debug,ubl_gate=debug`.

### URLs

- `UBL_PUBLIC_RECEIPT_ORIGIN` → se ausente, fallback:
  1. `UBL_RICH_URL_DOMAIN` (normalizado para `https://...`)
  2. `https://example.org`
- `UBL_PUBLIC_RECEIPT_PATH` → default `/r` (sempre com `/` inicial).
- `UBL_MCP_BASE_URL` → fallback:
  1. `UBL_API_BASE_URL`
  2. `UBL_API_DOMAIN` (normalizado para `https://...`)
  3. `https://api.example.org`

### Limits

- `UBL_CANON_RATE_LIMIT_ENABLED` → default `true` (`1|true|TRUE|yes|on`).
- `UBL_CANON_RATE_LIMIT_PER_MIN` → default `120`, mínimo `1`.
- `UBL_MCP_TOKEN_RPM` → default `120`, mínimo `1`.

### Write policy

- `UBL_WRITE_AUTH_REQUIRED` → default `false` (`1|true|TRUE|yes|on`).
- `UBL_WRITE_API_KEYS` → CSV (trim de itens).
- `UBL_PUBLIC_WRITE_WORLDS` → CSV; quando vazio, default:
  - `a/chip-registry/t/public`
  - `a/demo/t/dev`
- `UBL_PUBLIC_WRITE_TYPES` → CSV; quando vazio, default:
  - `ubl/document`
  - `audit/advisory.request.v1`


### LLM

- `UBL_ENABLE_REAL_LLM` → default `false` (`1|true|TRUE|yes|on`).
- `UBL_LLM_BASE_URL` → opcional (trim; vazio = `None`).
- `UBL_LLM_MODEL` → default `qwen3:4b` quando `UBL_LLM_BASE_URL` está setada; caso contrário `gpt-4o-mini`.
- `OPENAI_API_KEY` (**secret**) → opcional (trim; vazio = `None`).

### Crypto

- `UBL_CRYPTO_MODE` (string) → default `compat_v1`.
- Uso: compatibilidade e métricas; não há secret nesse campo e ele não deve carregar segredos.
- Validação: valores inválidos devem falhar cedo no bootstrap/config parse (não em tempo de assinatura).
- Observabilidade: o valor resolvido de `crypto_mode` aparece em logs redacted de bootstrap para facilitar diagnóstico sem expor segredos.

## Determinismo e caminho crítico

- O caminho crítico do pipeline/core deve receber configuração injetada (`PipelineConfig`/`AppConfig`) e **não** consultar ENV em produção.
- Wrappers legados de `*_from_env` permanecem apenas para compatibilidade e migração incremental; o uso recomendado para novos fluxos é injeção explícita.
- Regressões em bytes canônicos/outputs de vetores devem ser tratadas como breaking change de contrato.

### Build info

- `UBL_GENESIS_PUBKEY_SHA256` → opcional (trim; vazio = `None`).
- `UBL_RELEASE_COMMIT` → opcional (trim; vazio = `None`).
- `UBL_GATE_BINARY_SHA256` → opcional (trim; vazio = `None`).

## Referências cruzadas

- Contrato de config: `crates/ubl_config/src/lib.rs`.
- Consumo no serviço: `services/ubl_gate/src/main.rs` e `services/ubl_gate/src/lib.rs`.
- Operação: `ops/gate/README.md`.
