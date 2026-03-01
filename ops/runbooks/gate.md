# Runbook: UBL Gate

## Sintomas
- `/healthz` falha ou timeout.
- Aumento de erros 5xx.
- Queda de throughput do endpoint principal.

## Diagnóstico
1. Verificar saúde local:
   - `curl -fsS http://127.0.0.1:4000/healthz`
2. Ver logs:
   - `RUST_LOG=info` (ou nível superior para troubleshooting).
3. Conferir envs principais:
   - `UBL_GATE_BIND`, `UBL_DATA_DIR`, `UBL_STORE_BACKEND`, `UBL_STORE_DSN`, `RUST_LOG`.
4. Se disponível, conferir `/metrics` e latência p95/erro 5xx.

## Mitigação rápida
- Reiniciar processo/serviço (`docker compose restart` ou `systemctl restart ubl-gate`).
- Reduzir carga de entrada temporariamente.

## Resolução definitiva
- Corrigir config inválida em `ops/gate/env.example`/ambiente real.
- Corrigir backend de storage/DSN indisponível.
- Atualizar deployment com imagem válida.

## Rollback
- Reverter para última versão estável da imagem/binário.
- Restaurar arquivo de ambiente anterior.

## Postmortem checklist
- Linha do tempo.
- Causa raiz.
- Ações corretivas e preventivas.
- Atualização de SLO/error budget.

## SLO impact
Registrar indisponibilidade de `/healthz`, erro 5xx e impacto de latência p95 no período.
