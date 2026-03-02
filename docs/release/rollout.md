# Rollout P0 -> P1 (Chip-Native Governance)

**Status**: active  
**Owner**: Ops + Security  
**Last reviewed**: 2026-02-21

## Objetivo
Ativar e evoluir política exclusivamente por chips/receipts no próprio UBL, sem preflight externo como fonte de verdade.

## Fluxo canônico

1. Runtime inicia em P0 (genesis) com hash de runtime verificável.
2. Operação submete chips de preparação (`ubl/document`, `ubl/silicon.*`, artefatos de bootstrap).
3. Proposta de promoção para P1 é submetida como chip de governança.
4. CHECK/TR/WF aplicam regras e emitem receipt canônico.
5. Estado vigente de política passa a ser o último receipt válido dessa cadeia.

## Evidência obrigatória (on-ledger)

- Receipt CID da proposta de promoção (P0 -> P1).
- Receipt CID de aprovação/ativação.
- Trace completo (`/v1/receipts/:cid/trace`) da decisão.
- Witness externo do evento de promoção.

## Regra operacional

- Nenhuma decisão de rollout é considerada válida sem receipt.
- Scripts de preflight fora do UBL não são mais gate de produção.
- Scripts externos podem existir apenas para bootstrap físico do host e publicação de witness.

## Referência operacional

- `docs/ops/ROLLOUT_AUTOMATION.md`
