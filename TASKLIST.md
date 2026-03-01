# UBL Program Tasklist v2 (Pragmática e Completa)

**Status**: Active execution source of truth  
**Owner**: Core Runtime + Platform Engineering  
**Last reviewed**: 2026-02-24

## Contexto real (decisão de programa)

Estado atual:
- O repositório atual funciona, na prática, como `UBL-Plataforma-TESTE`.

Estado alvo:
- Extrair e congelar um `UBL-CORE` mínimo, estável e versionado (base compartilhada).
- Derivar três produtos/contextos a partir do core:
  - `UBL-Core-Mini` = core + particularidades de execução LLM/edge.
  - `UBL-Pessoa` = core + particularidades de soberania pessoal.
  - `UBL-Plataforma` = core + particularidades de coordenação coletiva/homeostase.

Princípio de controle:
- Nenhuma particularidade entra no `UBL-CORE`.
- Toda quebra de compatibilidade exige versionamento + migração + evidência.

---

## Regras de operação

- Contract-first: mudança de comportamento exige spec + teste de contrato + evidência no mesmo PR.
- Core-first: estabilizar núcleo antes de acelerar variantes.
- No bypass: mutação sempre via gate e pipeline canônico.
- Evidence-first: tudo relevante precisa gerar receipt/auditoria verificável.
- Honestidade operacional: lacunas ficam explícitas, com dono e critério de pronto.
- Task orchestration is chip-native: use `docs/ops/TASK_ORCHESTRATION_PROTOCOL.md` + `schemas/task.lifecycle.event.v1.json` for lifecycle transitions and evidence.

---

## Critério global de sucesso (programa)

- `UBL-CORE` definido por fronteira rígida, publicado e consumido por todos os derivados.
- `UBL-Core-Mini`, `UBL-Pessoa` e `UBL-Plataforma` rodando sobre o mesmo contrato de core.
- Busca/auditoria unificadas (fan-in dos stores) com resposta única e verificável.
- LLM advisor pessoal e advisor de plataforma isolados por identidade/chaves/tokens/stores/logs.
- LAB 512 com bootstrap oficial, evidência completa e rotina de operação estável.

---

## Orquestracao da Tasklist (imediato)

### Objetivo
Rodar a própria tasklist pelo pipeline UBL, com receipts em cada transição.

### Itens
- [ ] Ativar o protocolo `docs/ops/TASK_ORCHESTRATION_PROTOCOL.md`.
- [ ] Validar schema `schemas/task.lifecycle.event.v1.json` no KNOCK.
- [ ] Criar os primeiros chips para `L-01..L-05` com estado `open`.
- [ ] Executar um ciclo completo (`open -> in_progress -> done`) com evidência real.
- [ ] Publicar recibos em `artifacts/tasks/` e linkar no `TASKLIST.md`.

### DoD
- 5 transições reais registradas como chips, cada uma com receipt CID.

---

## Track 0 — Programa, naming e baseline

### Objetivo
Fechar nomenclatura, escopo e governança para parar drift de linguagem e prioridade.

### Itens
- [ ] Ratificar naming oficial de trabalho:
  - `UBL-Plataforma-TESTE` (estado atual)
  - `UBL-CORE` (núcleo compartilhado)
  - `UBL-Core-Mini` (derivado)
  - `UBL-Pessoa` (derivado)
  - `UBL-Plataforma` (derivado)
- [ ] Publicar mapa de repositórios e ownership por contexto.
- [ ] Definir branch/release policy por camada (core vs derivados).
- [ ] Definir política de breaking change para core (quem aprova, como migra).
- [ ] Fixar baseline de commit para início da extração.

### DoD
- Documento único de baseline aprovado e linkado em `docs/START-HERE-LLM-FIRST.md`.

---

## Track 1 — Fronteira rígida do UBL-CORE

### Objetivo
Definir exatamente o que pertence ao core e o que é extensão.

### In-scope obrigatório do core
- Envelope canônico (`@id`, `@type`, `@ver`, `@world`).
- Canon determinístico (rho/NRF/CID/UNC-1).
- Pipeline canônico (`KNOCK -> WA -> CHECK -> TR -> WF`).
- Receipt mínimo canônico.
- Contrato mínimo de AI Passport (identidade + limites + proveniência).
- Erros canônicos e semântica de falha.

### Itens
- [ ] Publicar matriz `CORE vs EXTENSION` (linha por capability).
- [ ] Definir `MUST NOT` explícitos para impedir poluição do core.
- [ ] Fixar versão de contrato inicial (`core-contract-v1`).
- [ ] Criar suite de conformance mínima obrigatória para qualquer derivado.

### DoD
- `CORE_BOUNDARY.md` aprovado + conformance mínima verde no CI.

---

## Track 2 — Lacunas canônicas (cirúrgicas)

### Objetivo
Fechar gaps que impedem auditabilidade plena.

### Itens
- [ ] Especificação normativa legível de `NRF-1.1`.
- [ ] Especificação semântica de `@world` (validação, escopo, interoperabilidade).
- [ ] Entrada explícita de AI Passport no mapa canônico (`CANON-REFERENCE`).
- [ ] Seção de composição entre `WASM_RECEIPT_BINDING_V1` e receipt canônico.
- [ ] Endurecer `WASM_CAPABILITY_MODEL_V1` para `fs_read` escopado + mapeamento de erro.

### DoD
- Cada lacuna com: spec + ponteiro de código + evidência + índice atualizado.

---

## Track 3 — Extração estrutural (de TESTE para CORE)

### Objetivo
Separar o que é núcleo do que é específico da plataforma atual.

### Itens
- [ ] Inventariar módulos atuais e classificar em `core`, `platform`, `shared-ext`, `legacy`.
- [ ] Extrair contratos e tipos de core para pacote/crate dedicado.
- [ ] Remover dependências circulares entre runtime e concerns de plataforma.
- [ ] Criar camada de extensão oficial (`extensions/` ou crates de domínio).
- [ ] Marcar componentes legados com plano de desativação.

### DoD
- Build do `UBL-CORE` sem depender de módulos platform-specific.

---

## Track 4 — Arquitetura de stores e auditoria unificada

### Objetivo
Parar a fragmentação operacional de consultas em múltiplas bases.

### Itens
- [ ] Documentar oficialmente os stores por contexto e papel (CAS/Event/Index/etc).
- [ ] Definir contrato de `auditoria unificada` (entrada única, fan-in interno, saída única).
- [ ] Implementar agregador de auditoria/search com reconciliação entre fontes.
- [ ] Padronizar resposta com: CIDs, contexto, estado de reconciliação, lacunas de certeza.
- [ ] Incluir rastreio de falha parcial de fonte (sem mascarar erro).

### DoD
- Um endpoint/comando único responde auditoria cross-store com prova verificável.

---

## Track 5 — LLM Engine e Advisors (pessoal vs plataforma)

### Objetivo
Implementar isolamento real entre agência pessoal e agência de plataforma.

### Itens
- [ ] Formalizar dois perfis de advisor:
  - `advisor.personal`
  - `advisor.platform`
- [ ] Garantir passaportes distintos por perfil.
- [ ] Garantir chaves/tokens/stores/logs segregados por perfil.
- [ ] Bloquear runtime para impedir advisor fora do scope/role.
- [ ] Definir hooks consultivos permitidos e proibidos por etapa do pipeline.
- [ ] Registrar proposal/advisory sempre com referência a input/receipt origem.

### DoD
- Testes de contrato provando que um advisor não consegue agir no contexto do outro.

---

## Track 6 — UBL-Core-Mini

### Objetivo
Produzir variante leve para bolso do LLM/edge mantendo contrato do core.

### Itens
- [ ] Definir perfil mínimo de runtime (`mini-profile-v1`).
- [ ] Definir dependências removíveis (sem quebrar invariantes).
- [ ] Garantir conformance core em footprint reduzido.
- [ ] Validar modo offline/degraded com receipts verificáveis.
- [ ] Publicar guia de integração para LLM Engine local.

### DoD
- `UBL-Core-Mini` passa conformance core e roda cenário de referência.

---

## Track 7 — UBL-Pessoa

### Objetivo
Criar derivado pessoal com soberania por padrão.

### Itens
- [ ] Definir bootstrap de identidade pessoal (keys, did/passport, world).
- [ ] Definir política local de privacidade e retenção.
- [ ] Implementar gestão de secrets e credenciais pessoais com rotação.
- [ ] Implementar comunicação federada com plataforma via chips/receipts.
- [ ] Definir controles de consentimento para ações cross-contexto.

### DoD
- UBL-Pessoa opera de forma autônoma e interopera com plataforma sem perder soberania.

---

## Track 8 — UBL-Plataforma (produção)

### Objetivo
Levar o atual TESTE para plataforma oficial estabilizada.

### Itens
- [ ] Aplicar contrato final de core nas superfícies da plataforma.
- [ ] Fechar protocolo de homeostase e conflito cross-contexto em runtime.
- [ ] Implementar NOC com visão de evidência e reconciliação (não só métricas).
- [ ] Fechar governança de policy e incidentes.
- [ ] Endurecer operação pública (edge, rate-limit, receipt URL model único).

### DoD
- Plataforma oficial roda em produção com compliance do core e evidência contínua.

---

## Track 9 — App de observabilidade (UI de operação)

### Objetivo
Reduzir operação por terminal e expor controle confiável via UI.

### Itens
- [ ] Definir escopo MVP do app como NOC operacional da plataforma.
- [ ] Integrar backend Rust (orquestração segura de CLI + APIs + WebSocket).
- [ ] Entregar painéis mínimos:
  - Home (health geral)
  - Status de componentes
  - Registry/Chips
  - Auditoria unificada
  - LLM Gateway status
- [ ] Implementar modelo frente/verso com settings por painel/componente.
- [ ] Garantir trilha auditável para toda ação iniciada na UI.

### DoD
- Fluxos críticos de operação executáveis sem terminal em rotina normal.

---

## Track 10 — Segurança operacional e cerimônias

### Objetivo
Fechar hardening de identidade, segredos e resposta a incidente.

### Itens
- [ ] Checklist de key ceremony (machine birth + key birth).
- [ ] Política de trust anchors e attestation pinning.
- [ ] Política de break-glass com trilha obrigatória.
- [ ] Política de segregação operador/admin.
- [ ] Validação de não vazamento de segredos em artifacts/logs.
- [ ] Exercício de incidente com evidência e postmortem.

### DoD
- Pacote de segurança operacional aprovado e ensaiado.

---

## Track 11 — Qualidade, conformance e release

### Objetivo
Impedir regressão silenciosa e release sem prova.

### Itens
- [ ] CI WF verde com contract + conformance + invariantes.
- [ ] Reproducibilidade/attestation para commit alvo.
- [ ] Checklist de promoção LAB 256 -> LAB 512 com signoff.
- [ ] Gate de release por contexto (core/mini/pessoa/plataforma).
- [ ] Artefatos de evidência preservados por release.

### DoD
- Nenhuma promoção sem evidência executável anexada.

---

## Track 12 — LAB 256 rehearsal e LAB 512 bootstrap

### Objetivo
Executar caminho completo sem exceção manual.

### Itens
- [ ] Rehearsal completo em LAB 256 (clean bootstrap).
- [ ] Validar receipts, witnesses e sincronização dual-plane.
- [ ] Rodar teste de aceitação Episode 1 (Small/Big).
- [ ] Rodar simulação de incidente + recuperação.
- [ ] Capturar pacote de evidência para go/no-go.
- [ ] Executar bootstrap oficial LAB 512.
- [ ] Emitir receipt inaugural + trust anchors públicos.
- [ ] Congelar snapshot de genesis.

### DoD
- LAB 512 operacional com trilha histórica verificável desde genesis.

---

## Track 13 — Pós-bootstrap e continuidade

### Objetivo
Transformar bootstrap em operação contínua confiável.

### Itens
- [ ] Heartbeat receipts agendados.
- [ ] Backups criptografados com teste de restore.
- [ ] Baseline de monitoramento e alertas revisado.
- [ ] Revisão semanal de lacunas e débito arquitetural.
- [ ] Abrir próximo ciclo com metas trimestrais.

### DoD
- Operação estável por 30 dias com incidentes tratados via protocolo.

---

## Ordem executiva (critical path)

1. Track 0 (baseline e naming)
2. Track 1 (fronteira do core)
3. Track 2 (lacunas canônicas)
4. Track 3 (extração estrutural)
5. Track 10 (segurança operacional mínima)
6. Track 11 (gates de qualidade/release)
7. Track 12 (LAB 256 -> LAB 512)
8. Track 13 (estabilização)
9. Tracks 6/7/8 em paralelo controlado após core congelado
10. Track 9 (app observabilidade) evolui em paralelo, sem furar contrato do core

---

## Matriz de dependências (resumo)

- Track 6/7/8 dependem de Track 1 + 2 + 3.
- Track 12 depende de Track 10 + 11.
- Track 9 depende de Track 4 + 5 para fluxo de auditoria e advisor corretos.
- Track 4 depende de definição de stores e contratos mínimos de receipt/canon.

---

## Backlog explícito de lacunas (L-series)

| ID | Lacuna | Prioridade | Dono sugerido | Critério de pronto |
|---|---|---|---|---|
| L-01 | NRF-1.1 sem spec normativa legível | P0 | Core Runtime | `docs/canon/NRF-1.1.md` ativo + conformance |
| L-02 | `@world` sem spec semântica formal | P0 | Core Runtime | `docs/canon/WORLD.md` + validação runtime |
| L-03 | AI Passport fora do mapa canônico oficial | P0 | Identity/Runtime | entrada dedicada no `CANON-REFERENCE` |
| L-04 | Binding WASM receipt vs canonical receipt incompleto | P1 | Runtime + WASM | seção de composição + testes |
| L-05 | `fs_read` scoped semantics em WASM capability incompleto | P1 | WASM Runtime | regra runtime + error mapping validado |
| L-06 | Busca/auditoria ainda fragmentada em múltiplos stores | P0 | Platform Data | agregador fan-in em produção |
| L-07 | Separação advisor personal/platform ainda parcial | P0 | Identity + Runtime | testes de isolamento em CI |

---

## Fora deste ciclo (explicitamente)

- Features cosméticas de UI sem impacto operacional.
- Novas superfícies de protocolo sem impacto no core/release.
- Otimizações prematuras antes de conformance estável.

---

## Cadência de revisão

- Revisão executiva: semanal.
- Revisão de lacunas L-series: 2x por semana.
- Atualização de status: toda mudança relevante de fase.
- Regra: item fechado sem evidência = item reaberto.
