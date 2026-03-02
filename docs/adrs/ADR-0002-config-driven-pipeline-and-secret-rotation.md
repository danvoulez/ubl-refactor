# ADR-0002: Pipeline config-driven e rotação de segredos sem mutação de ENV

- Data: 2026-03-02
- Status: Accepted

## Contexto
Historicamente, partes do core/pipeline consultavam variáveis de ambiente em tempo de execução para decidir `crypto_mode` e para gerenciar rotação de segredos de stage. Esse padrão introduz risco de drift entre ambientes, reduz determinismo operacional e dificulta troubleshooting reproduzível.

Também havia fluxo de rotação com mutação de ENV (`set_var`) durante runtime, o que não escala bem em processos concorrentes e torna invariantes de segurança mais frágeis.

## Decisão
1. Tornar pipeline/core **config-driven**:
   - `CryptoMode` e demais flags críticas passam por parse/validação de configuração no bootstrap.
   - O caminho crítico recebe configuração injetada (`PipelineConfig`/`AppConfig`) em vez de consultar ENV ad hoc.

2. Manter wrappers legados `*_from_env` apenas para compatibilidade/migração:
   - Não recomendados para novos fluxos.
   - Uso permitido como shim temporário para evitar quebra de consumidores.

3. Rotação de segredos sem mutação de ENV:
   - Estado corrente/anterior em memória com sincronização (`Arc<RwLock<...>>`).
   - Persistência coordenada via `DurableStore` para continuidade operacional.
   - Invariante: lock curto; não segurar lock durante `await`/I/O.

## Consequências
- Menor risco de drift por ENV e maior determinismo entre ambientes.
- Observabilidade mais clara: `crypto_mode` resolvido pode ser registrado em logs redacted de bootstrap.
- Rollback mais previsível (reversão por commit/config), sem dependência de mutações runtime no processo.
- Custo: coexistência temporária com shims legados até migração completa.

## Guardrails de contrato
- Não alterar contratos HTTP do Gate.
- Não alterar bytes canônicos nem outputs de vetores.
- Regressões nesses pontos devem ser tratadas como quebra de contrato e bloquear rollout.

## Referências
- `docs/reference/config.md`
- `docs/operations/stage-secret-rotation.md`
- `ops/runbooks/gate.md`
