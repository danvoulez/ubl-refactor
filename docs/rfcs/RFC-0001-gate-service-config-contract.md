# RFC-0001: Gate as Service + Config Contract

## Objetivo
Formalizar o Gate como serviço HTTP com contrato de configuração centralizado.

## Contexto
O repositório já consolidou `ubl_gate` e `ubl_config` como caminho principal.

## Proposta
- Gate sobe por `make gate` e expõe `/healthz`.
- Contrato de env/default é derivado de `crates/ubl_config`.
- Documentação oficial de config fica em `docs/reference/config.md`.

## Alternativas
- Manter envs dispersos em múltiplos READMEs (rejeitado por drift).

## Impacto (API/compat)
Sem alteração de runtime/API.

## Segurança/privacidade
Sem nova superfície; apenas documentação de contrato já existente.

## Observabilidade
Padroniza uso de `RUST_LOG` e referência para `/metrics` quando habilitado no Gate.

## Rollout & rollback
Rollout imediato por documentação; rollback é reverter este RFC e referências.

## Migração
Times devem trocar links antigos por `docs/index.md` e `docs/reference/config.md`.

## Questões abertas
Definir catálogo formal de dashboards em doc dedicado de observabilidade.
