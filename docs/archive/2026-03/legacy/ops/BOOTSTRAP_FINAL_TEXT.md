# UBL Final Position (Pragmatic)

**Status**: final draft  
**Date**: 2026-02-22  
**Scope**: LAB 512 readiness + Episode 1 operating model

## Síntese executiva

O valor desta fase foi a combinação de:

1. Intervenção do Gemini (hardening de confiança e operação).
2. Comentários finais (decisões práticas de execução).

Resultado: menos metáfora, mais contrato técnico e trilha auditável.

## Decisão central

O bootstrap oficial não será improvisado no LAB 512.

- LAB 256: laboratório de validação e ensaio completo.
- LAB 512: gênese e produção histórica, com mudança mínima.

Sem evidência completa no 256, não há promoção para o 512.

## Episode 1 (televisionável e científico)

Objetivo: provar que dá para combinar motor determinístico + aconselhamento LLM sem perder confiança.

Prova final em três camadas:

1. Receipts + ledgers (cadeia auditável).
2. Lineage/proveniência (execução rastreável).
3. Vídeo hasheado e selado no dossiê.

## Definição oficial de papéis

### `ubl-0` (Small, control plane)

Faz:

- método/governança (juiz),
- coordenação de episódio,
- publish/archive,
- montagem de bundle final,
- TV/OBS e narrativa operacional.

Não faz:

- execução pesada de simulação/dataset.

### `UBL-0` (Big, data plane)

Faz:

- execução pesada determinística,
- ingestão multi-plataforma,
- produção de outputs e receipts de pesquisa,
- telemetria de execução para auditoria.

Não faz:

- criação de método,
- decisão de governança,
- “TV”/camada editorial.

## Entra/Sai pragmático

### Entra no Small

- engine de episódio (state machine),
- governança de publish/archive,
- integração OBS,
- recepção e consolidação de evidências do Big.

### Sai do Small

- workload pesado de execução/simulação.

### Entra no Big

- execução determinística endurecida (fuel, perfil estável, policy restrita),
- pipeline de ingestão multi-plataforma,
- emissão de evidências de execução para o Small.

### Sai do Big

- UI/editorial,
- lógica de comitê/decisão de episódio.

## Correção técnica consolidada

1. WASM já roda “por dentro” do TR no runtime atual.
2. Comitê LLM nativo completo ainda é evolução do Small (não está fechado no estado atual).

## Regras fixas

1. URL pública de recibo: `https://logline.world/r#ubl:v1:<token>`.
2. Contract-first + conformance obrigatórios.
3. Sem `hard error` sem receipt nos fluxos relevantes.
4. Backup, snapshot e freeze manifest desde o primeiro ciclo oficial.

## Próximo passo único

Executar a `TASKLIST.md` nova de LAB 512 com gate de promoção 256 -> 512.
