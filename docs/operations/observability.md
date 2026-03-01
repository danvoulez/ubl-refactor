# Observability — UBL Gate

## Logs

- Controle por `RUST_LOG`.
- Recomendação: `info` em produção; elevar para `debug` apenas em troubleshooting limitado.

## Tracing

- O Gate usa stack Rust com tracing/logging configurável por ambiente.
- Propague contexto de request quando disponível no pipeline.

## Métricas

- Endpoint de métricas: `/metrics` (quando exposto pelo serviço).
- Métricas mínimas para dashboard:
  - disponibilidade `/healthz`,
  - latência p95,
  - taxa de erro 5xx,
  - volume de requests.

## Dashboards sugeridos

1. **Gate Golden Signals**: tráfego, latência, erros, saturação.
2. **SLO Board**: burn-rate e consumo de error budget.
3. **Deploy Health**: comparação pré/pós rollout.
