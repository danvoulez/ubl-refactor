# SLO — UBL Gate

## SLIs

1. **Disponibilidade `/healthz`**
   - SLI: proporção de checks HTTP 200 em janelas de 30 dias.
2. **Latência p95**
   - SLI: p95 por endpoint principal monitorado.
3. **Taxa de erro 5xx**
   - SLI: percentual de respostas 5xx sobre total de requests.

## Objetivos (SLO)

- Disponibilidade `/healthz`: **99.9%** (30 dias).
- Erro 5xx: **< 1.0%** por janela diária e mensal.
- Latência p95: acompanhar por endpoint e manter baseline acordado pelo time de ops.

## Error budget

- Budget mensal de indisponibilidade para `/healthz`: ~43m12s.
- Se consumir >50% do budget antes da metade da janela:
  - congelar mudanças não essenciais,
  - priorizar correções de confiabilidade,
  - revisar runbooks e alertas.
