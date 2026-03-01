# UBL ARCHITECTURE v2

## 1. Prólogo

UBL nasce de uma recusa simples:
não aceitar sistemas inteligentes sem memória verificável.

Se uma ação muda o mundo, ela precisa deixar rastro.
Se deixa rastro, precisa ser canônico.
Se é canônico, precisa ser auditável por qualquer parte legítima.
Essa é a linha que separa discurso de engenharia.

UBL não é uma interface.
UBL não é um relatório de compliance.
UBL é um contrato operacional de verdade:
bytes determinísticos, identidade explícita, decisão rastreável.

## 2. Constituição do Sistema (Normativa)

Os itens abaixo são invariantes constitucionais.

1. Determinismo de conteúdo é inviolável.
Mesmo input canônico + mesma versão de regras => mesmo Chip CID.

2. Receipt é evento, não conteúdo.
O mesmo chip pode gerar receipts diferentes em execuções distintas.

3. Nenhuma mutação existe fora da pipeline.
Sem bypass administrativo, técnico ou “temporário”.

4. O Gate é a única fronteira de entrada de mutação.
Toda escrita entra como chip.

5. O Canon é único no trust path.
Hash, assinatura e prova derivam de NRF-1.1.

6. O LLM é advisor responsável, não árbitro final.
Pipeline decide. Advisory assina e responde pelo que diz.

7. AI Passport é perfil operacional, não ornamento.
Identidade, proveniência, limites e deveres executáveis.

8. UBL é fractal em três contextos.
Core portátil, Pessoal soberano, Plataforma interoperável.

9. Advisors pessoal e de plataforma são isolados por design.
Chaves, tokens, stores e logs distintos.

10. Plataforma é motor coletivo de verdade e homeostase.
Coordena prova e continuidade sem sequestrar soberania pessoal.

### 2.1 Convenções Normativas
- MUST / MUST NOT: requisito absoluto.
- SHOULD / SHOULD NOT: recomendação forte com exceções justificáveis.
- MAY: opção permitida.
- Terminologia normativa segue RFC 2119.

## 3. Modelo Fractal: Core, Pessoal, Plataforma

A mesma verdade, três escalas de operação.

### 3.1 UBL-Core
O UBL-Core é o núcleo portátil.
Ele não depende de moda, interface ou contexto comercial.
Ele define:
- canon,
- CID,
- envelope,
- semântica mínima de pipeline,
- regras de validação que não podem negociar com conveniência.

O Core é pequeno de propósito.
Tudo que entra nele vira compromisso de longo prazo.

### 3.2 UBL Pessoal
O UBL Pessoal é a soberania em execução.
Não é uma feature premium: é a unidade básica da dignidade operacional.

No UBL Pessoal:
- a pessoa tem seu advisor dedicado,
- o advisor trabalha sob AI Passport próprio,
- os dados e rastros pertencem ao domínio pessoal,
- a autonomia não depende de permissão da Plataforma.

### 3.3 UBL Plataforma
A UBL Plataforma é o espaço de interoperabilidade.
Ela não substitui soberanias pessoais; ela as conecta com prova.

Funções da Plataforma:
- receber interações entre domínios como chips,
- garantir verificabilidade comum por receipts,
- operar homeostase quando há conflito, ruído ou fraude,
- manter continuidade operacional da rede.

### 3.4 Fronteiras Obrigatórias (MUST)
- Core, Pessoal e Plataforma MUST compartilhar contrato canônico.
- Advisor pessoal e Platform advisor MUST ter passaportes distintos.
- Chaves/tokens/stores/logs MUST ser segregados por contexto.
- Nenhum contexto MAY simular identidade de outro.
- A Plataforma MUST atuar como nó de coordenação, não como dona da soberania pessoal.

### 3.5 Resolução de Conflito Cross-Contexto
Quando dois contextos soberanos produzem cadeias de evidência contraditórias:
- conflito MUST ser detectado por reconciliação automatizada;
- ambas cadeias MUST ser preservadas integralmente no ledger;
- resolução MUST gerar receipt próprio com referência a ambas origens;
- nenhum contexto MAY resolver conflito unilateralmente apagando evidência do outro.

---

## 4. Pipeline Canônico: KNOCK -> WA -> CHECK -> TR -> WF

Pipeline é onde intenção vira fato auditável.

### 4.1 KNOCK
Porta dura.
Valida forma, limites e sanidade canônica antes de qualquer efeito.
Input: raw request envelope.
Output: validated envelope OR rejection with structured error.
Se falhar aqui, falha cedo, sem teatro.

### 4.2 WA
Declara autorização de trabalho.
Quem pede, em qual mundo, sob quais limites.
Input: validated envelope from KNOCK.
Output: work authorization token (identity + world + scope + limits).
Sem WA válido, não existe legitimidade operacional.

### 4.3 CHECK
Governança executável.
Policy deixa de ser texto e vira decisão reproduzível.
Input: WA token + chip payload.
Output: policy decision (allow/deny/conditional) + dependency graph.
CHECK decide permissões, dependências e bloqueios.

### 4.4 TR
Transição determinística.
Execução com contrato puro (entrada canônica -> saída canônica), fuel e rastreio.
Input: authorized chip + fuel budget.
Output: canonical state transition + execution trace.
Sem I/O arbitrário disfarçado de “atalho”.

### 4.5 WF
Fechamento e prova.
Finaliza receipt, persiste evidência, encadeia integridade.
Input: completed transition + trace.
Output: finalized receipt + persisted evidence + integrity chain link.
WF não é pós-processo opcional; é parte da verdade do evento.

### 4.6 Leis de Operação (MUST)
- Toda mutação MUST atravessar os cinco estágios.
- Nenhum estágio MAY ser pulado por privilégio interno.
- Falhas MUST produzir erro explícito e auditável no ponto de falha.
- Receipts MUST carregar contexto suficiente para reconstrução forense.
- Observabilidade MUST refletir o pipeline real, não uma narrativa paralela.

### 4.7 Alma do Pipeline
Pipeline não existe para burocratizar ação.
Existe para que ação com impacto não seja invisível.

No UBL, velocidade sem prova é ruído.
Prova sem ação é museu.
Pipeline é o pacto entre as duas.

## 5. Canon, CID e Verdade de Conteúdo

Toda arquitetura séria escolhe onde mora a verdade.
No UBL, a verdade mora no conteúdo canônico.

### 5.1 Layer 0 — Machine Canon (NRF-1.1)
Layer 0 é o chão de concreto:
- bytes determinísticos,
- estrutura sem ambiguidade,
- base única para hash, assinatura e armazenamento.

O sistema pode ter mil interfaces.
No fim, só uma coisa decide igualdade: os bytes canônicos.

### 5.2 Layer 1 — Anchored JSON
Layer 1 existe para autoria, leitura e operação por humanos/LLMs.
Ele precisa ser claro, previsível e ancorado:
- `@type`
- `@id`
- `@ver`
- `@world`

Layer 1 é a linguagem de trabalho.
Layer 0 é a linguagem da prova.

### 5.3 CID (LOCKED)
CID de chip:
`b3:` + `blake3(nrf_bytes)` em hex lowercase, 64 caracteres (256 bits completos).
Truncamento de CID é proibido em qualquer contexto de confiança.

Sem exceção semântica.
Sem “equivalência aproximada”.
Sem hash alternativo por conveniência local.

### 5.4 PF-01 e PF-02
PF-01:
mesmo conteúdo canônico => mesmo Chip CID.

PF-02:
receipt é evento situado no tempo/execução => Receipt CID pode variar para o mesmo chip.

Conclusão operacional:
- conteúdo se compara por Chip CID,
- execução se analisa por receipt + trilha de contexto.

### 5.5 UNC-1 e números
Número é fonte clássica de ambiguidade.
UBL resolve isso no contrato:
- inteiros simples quando cabíveis,
- demais casos via UNC-1,
- sem float cru no caminho de confiança.

### 5.6 Invariantes (MUST)
- Hash/sign/store MUST derivar de Layer 0.
- Canon MUST rejeitar entradas ambíguas (duplicata de chave, UTF inválido, etc.).
- Quebra de canon MUST ser tratada como erro estrutural, não warning.
- Mudança breaking em canon MUST implicar versionamento formal e trilha de migração.

### 5.7 Por que isso importa
Sem canon, cada nó lembra uma versão diferente do mundo.
Com canon, divergência vira bug detectável.

Canon não reduz criatividade.
Ele reduz disputa inútil sobre fatos básicos.

### 5.8 Exemplo de Migração de Canon
Quando uma mudança breaking no canon é inevitável:
1. nova versão recebe identificador explícito (e.g., NRF-1.2);
2. chips existentes mantêm CID original sob versão de origem;
3. período de convivência definido com dual-validation;
4. após período, versão antiga passa a read-only;
5. todo o processo gera trilha de migração auditável.

---

## 6. AI Passport como UBL-Core-mini

AI Passport começou como shape de compliance.
No v2 ele é assumido pelo que realmente precisa ser:
perfil operacional completo de um UBL-Core-mini para o LLM.

### 6.1 O que ele é
AI Passport define:
- quem o LLM é (identidade operacional),
- o que ele pode fazer (rights/scope),
- pelo que responde (duties/proveniência),
- até onde pode ir (fuel/limites),
- como prova autoria (cadeia de assinatura).

Não é decoração de metadado.
É contrato executável.

### 6.2 Três blocos constitutivos
Identidade:
- DID/chave/provedor/modelo com estabilidade verificável.

Proveniência:
- advisory sempre referenciado ao input/receipt de origem.
- sem output “solto”.

Dignidade operacional:
- limites claros, aplicados pelo pipeline, iguais para qualquer ator equivalente.
- nem licença irrestrita, nem contenção arbitrária.

Campos mínimos obrigatórios do AI Passport:
- `passport_id`: identificador único do passaporte
- `did`: identidade descentralizada do advisor
- `provider`: provedor do modelo
- `model`: modelo/versão em execução
- `role`: personal | platform
- `rights`: lista de ações autorizadas
- `duties`: obrigações de proveniência e evidência
- `scope`: fronteira operacional (world/contexto)
- `fuel`: orçamento computacional/financeiro
- `signing_key_ref`: referência à chave de assinatura ativa
- `issued_at`: timestamp de emissão
- `expires_at`: timestamp de validade

### 6.3 Consequência arquitetural
Todo advisor que opera no ecossistema UBL precisa de passaporte próprio.
Sem passaporte válido, não existe agência legítima no runtime.

### 6.4 Pessoal vs Plataforma
- Advisor pessoal: lealdade ao interesse da pessoa.
- Advisor da plataforma: lealdade ao interesse sistêmico da rede.

Esses dois papéis são complementares, não intercambiáveis.
Compartilhar identidade entre eles viola a arquitetura.

### 6.5 Enforcement (MUST)
- Runtime MUST validar passaporte antes de ações de advisor.
- Rights/duties/scope/fuel MUST ser aplicados em execução.
- Outputs de advisor MUST ser assinados e auditáveis.
- Narrativa de advisor MUST permanecer não-soberana frente à decisão da pipeline.

### 6.6 Tese central da Bandeira #1
O ponto não é “humanizar” LLM.
O ponto é impedir irresponsabilidade estrutural.

Sem identidade: ninguém responde.
Sem proveniência: ninguém verifica.
Sem limites dignos: ninguém confia.

AI Passport como UBL-Core-mini é a forma de juntar os três sem sacrificar engenharia.

## 7. LLM Engine: Advisor, não soberano

O LLM Engine existe para ampliar entendimento e ação assistida.
Ele não existe para reescrever a realidade do protocolo.

### 7.1 Papel canônico
O Engine:
- interpreta contexto,
- propõe,
- classifica,
- narra,
- roteia.

A pipeline:
- decide,
- registra,
- prova.

Essa separação não é estética. É anticolapso.

### 7.2 Hook points operacionais
Hooks aceitáveis (consultivos — output não altera decisão da pipeline):
- pós-CHECK: explicação de decisão,
- pós-TR: resumo de execução,
- pós-WF: classificação/roteamento,
- on-demand: narrativa de auditoria.

Hooks proibidos:
- alterar decisão já tomada,
- emitir CID “de autoridade” fora do fluxo canônico,
- mutar estado sem submissão formal como chip.

### 7.3 Modo de falha
Falha de advisor não pode quebrar o núcleo determinístico.
Quando advisor falha:
- registra-se o erro,
- preserva-se rastreabilidade,
- segue-se operação do core conforme política.

### 7.4 Qualidade e custo
Engines podem variar (local/premium/híbrido), mas:
- identidade e proveniência não variam,
- contrato de evidência não varia,
- fronteiras de soberania não variam.

Trocar modelo é detalhe de implementação.
Quebrar trilha auditável é quebra de arquitetura.

### 7.5 Requisitos normativos (MUST)
- Engine MUST operar sob AI Passport válido.
- Todo advisory MUST referenciar input/receipt de origem.
- Engine MUST NOT atuar como atalho de mutação.
- Saída de advisor MUST ser tratada como evidência consultiva, não decisão final.
- Isolamento entre advisors pessoal/plataforma MUST ser preservado em runtime.

### 7.6 Intenção
LLM Engine no UBL não é “oráculo”.
É trabalhador especializado com carteira assinada no protocolo.

---

## 8. Gate Único e Superfícies (HTTP/MCP)

No UBL, múltiplas interfaces podem existir.
Múltiplas portas de mutação, não.

### 8.1 Gate como fronteira institucional
`ubl_gate` é a fronteira única de escrita.
Qualquer operação com efeito precisa cruzar essa fronteira como chip.

Sem bypass de debug.
Sem endpoint secreto de emergência.
Sem “aplicação interna” com privilégio extracontratual.

### 8.2 HTTP e MCP
HTTP e MCP são duas superfícies do mesmo compromisso:
- mesma política,
- mesma trilha de auditoria,
- mesma obrigação de passagem pelo pipeline.

Se o protocolo por baixo diverge, não é outra interface.
É outro sistema.

### 8.3 Mapeamento MCP (normativo)
Chamadas MCP de escrita:
- MUST equivaler semanticamente a submissão de chip.

Chamadas MCP de leitura:
- SHOULD mapear para endpoints read-only e relatórios auditáveis.

Ferramentas MCP que executam ações:
- MUST produzir evidência rastreável (evento/receipt/artefato), sem lacunas.

### 8.4 Idempotência e previsibilidade
Gate precisa ser forte em repetição e falha:
- mesma intenção com mesma chave de idempotência => comportamento consistente,
- erro deve ser explícito e recuperável,
- retry não pode gerar realidade duplicada silenciosa.

### 8.5 Requisitos normativos (MUST)
- Toda mutação MUST entrar por gate.
- Toda mutação MUST virar chip.
- Toda mutação MUST passar por KNOCK→WA→CHECK→TR→WF.
- Toda resposta de mutação MUST apontar para prova verificável (receipt/trace/evidência correlata).
- Nenhuma superfície MAY escapar de policy enforcement do CHECK.

### 8.6 Consequência prática
Gate único não reduz flexibilidade.
Ele reduz entropia institucional.

Sem gate único, cada integração inventa seu próprio “quase-UBL”.
Com gate único, diversidade de clientes convive com uma única verdade operacional.

## 9. Receipts, Ledger e Auditoria Unificada

No UBL, buscar e auditar não são mundos separados.
Buscar já deve retornar evidência com contexto suficiente para decisão.

### 9.1 Receipt como unidade de prova
Receipt é a narrativa técnica do evento:
- o que entrou,
- por onde passou,
- o que decidiu,
- com quais limites e assinaturas.

Sem receipt, há execução.
Com receipt, há responsabilidade.

Campos mínimos de um receipt:
- `receipt_cid`: CID do receipt (PF-02: pode variar por execução)
- `chip_cid`: CID do chip de origem (PF-01: determinístico)
- `pipeline_trace`: estágios atravessados com timestamps
- `policy_decision`: resultado do CHECK
- `transition_hash`: hash do estado resultante
- `actor_passport_id`: passaporte do ator principal
- `advisor_passport_id`: passaporte do advisor (se envolvido)
- `signatures`: assinaturas relevantes
- `world`: contexto de execução
- `created_at`: timestamp de finalização

### 9.2 Ledger como memória contínua
Ledger é append-only por princípio.
Correção não apaga história; adiciona capítulo com vínculo explícito.

Isso preserva:
- reprodutibilidade forense,
- confiança institucional,
- capacidade de explicar o sistema sob estresse.

### 9.3 Auditoria unificada
Operacionalmente, o usuário não deveria “consultar quatro lugares”.
O sistema deve fazer fan-in e devolver resposta única, concisa e verificável.

Uma consulta útil precisa combinar, quando aplicável:
- visão de eventos,
- visão de receipts,
- visão de artefatos/chips,
- estado de reconciliação entre stores.

### 9.4 Linguagem comum de auditoria
Toda saída de auditoria relevante deve carregar:
- identidade do contexto (`@world`),
- referências de prova (CIDs),
- estado de reconciliação,
- limites de certeza e lacunas observadas.

Auditoria sem grau de reconciliação vira opinião.
Auditoria com reconciliação vira engenharia.

### 9.5 Requisitos normativos (MUST)
- Operações de auditoria MUST preservar rastreabilidade fim a fim.
- Respostas de busca/auditoria MUST expor reconciliação entre fontes relevantes.
- Ledger MUST permanecer append-only.
- Evidência MUST ser referenciável por CID e contexto.
- Falhas parciais de fonte MUST ser explicitadas, não mascaradas.

### 9.6 Homeostase por evidência
Quando há conflito, o sistema não escolhe narrativa:
ele reconstrói cadeia de evidências e mostra o estado da verdade operacional.

---

## 10. Stores e Fronteiras de Soberania

Store não é só infra.
Store define quem pode afirmar o quê sobre qual realidade.

### 10.1 Papéis de store
- ChipStore/CAS: guarda conteúdo e artefatos por endereçamento de conteúdo.
- Durable/Event layers: preservam evolução operacional e replay auditável.
- Índices e relatórios: tornam evidência navegável sem quebrar contrato de origem.

Esses papéis cooperam, mas não se confundem.

### 10.2 Separação por contexto
A arquitetura fractal exige segregação real entre:
- contexto pessoal,
- contexto de plataforma,
- contextos institucionais adicionais.

Separar apenas por convenção de nome é insuficiente.
A separação MUST ser enforced em runtime por:
- namespaces criptograficamente distintos (chaves independentes por contexto);
- tokens não-transferíveis entre contextos;
- trilhas de auditoria segregadas por store;
- política de acesso avaliada por contexto no CHECK.

### 10.3 Advisor boundary
Advisor pessoal e advisor de plataforma:
- não compartilham passaporte,
- não compartilham chaves,
- não compartilham trilhas como se fossem um ator único.

A fronteira aqui é ética e técnica ao mesmo tempo.

### 10.4 Soberania e interoperabilidade
Interoperar não significa centralizar.
A Plataforma coordena o trânsito de prova entre soberanias.
Ela não absorve soberanias.

Esse desenho permite cooperação sem apagar autonomia local.

### 10.5 Requisitos normativos (MUST)
- Stores MUST ser segregados por contexto de soberania.
- Credenciais/chaves/tokens MUST ser distintos entre pessoal e plataforma.
- Política de acesso MUST refletir essa segregação.
- Operações cross-context MUST produzir trilha auditável explícita.
- Nenhum contexto MAY assumir identidade de outro para simplificar operação.

### 10.6 Consequência prática
Sem fronteiras claras, escala vira confusão.
Com fronteiras claras, escala vira federação verificável.

## 11. Homeostase da Plataforma (Bandeira 3)

A Plataforma não existe para mandar.
Existe para manter o ecossistema respirando com verdade verificável.

### 11.1 Definição operacional
Homeostase, no UBL, é a capacidade de manter estabilidade de confiança sob mudança contínua:
- novos eventos,
- falhas parciais,
- conflito entre versões de narrativa,
- pressão de tempo e custo.

Estabilidade aqui não é rigidez.
É continuidade auditável com correção explícita.

### 11.2 Verdade coletiva
“Verdade coletiva” não significa consenso político.
Significa capacidade compartilhada de verificar:
- fato,
- contexto,
- autorização,
- resultado.

A unidade dessa verdade é a evidência encadeada, não a opinião dominante.

### 11.3 Mecanismos de homeostase
A Plataforma sustenta homeostase por:
- canon único (reduz ambiguidade estrutural),
- gate único (reduz bypass institucional),
- policy executável (reduz arbitrariedade),
- auditoria unificada (reduz fragmentação operacional),
- ledger append-only (preserva memória sob conflito).

### 11.3.1 Protocolo de Resolução de Conflito
Quando evidências de contextos distintos divergem, a Plataforma MUST:
1. detectar divergência por reconciliação periódica ou on-demand;
2. preservar ambas cadeias de evidência (ledger append-only);
3. emitir chip de conflito com referência cruzada às origens;
4. produzir receipt de reconciliação com resultado e trilha de decisão;
5. impedir apagamento unilateral de evidência.

Protocolo normativo completo: ver §11.A.

### 11.A Protocolo Federado de Conflitos (Agregado)

Conflito em federação não é exceção: é condição normal de operação.
O requisito constitucional é resolver sem apagar evidência e sem colapsar soberania.

### 11.A.1 Taxonomia de conflito
Toda divergência cross-context MUST ser classificada em uma das classes:
- `TEMPORAL`: mutações válidas com ordem ambígua;
- `SEMANTICO`: interpretações incompatíveis do mesmo evento;
- `AUTORIDADE`: sobreposição de jurisdição/escopo;
- `INTEGRIDADE`: quebra/adulteração/inconsistência da cadeia de evidência.

### 11.A.2 Detecção e contenção proporcional
- Detecção MUST gerar chip de conflito via gate com referências cruzadas às cadeias envolvidas.
- Objeto contestado MUST entrar em contenção proporcional por política (`block`, `queue` ou `read-only warning`).
- Contenção MUST ter prazo máximo configurado; expiração sem resolução MUST escalar automaticamente.

### 11.A.3 Escada de resolução
Resolução segue escada de autonomia decrescente:
1. `AUTOMATICO`: regra determinística já definida;
2. `POLICY-ASSISTED`: CHECK aplica política versionada;
3. `ADVISOR-PROPOSED`: advisor propõe, humano decide;
4. `HUMAN-DECIDED`: decisor autorizado resolve com trilha;
5. `NOC-ESCALATED`: incidente operacional para conflito de alto impacto.

Notas normativas:
- `INTEGRIDADE` MUST poder escalar diretamente para `NOC-ESCALATED`.
- Estratégias temporais MUST usar ordenação canônica definida por política (não dependente apenas de relógio local).

### 11.A.4 Receipt de reconciliação
Toda resolução MUST produzir receipt de reconciliação com, no mínimo:
- `conflict_id`
- `conflict_class`
- `resolution_step` (1-5)
- `evidence_a_cid`
- `evidence_b_cid`
- `outcome`
- `resolved_at`

Campos condicionais por degrau:
- `policy_ref` (quando houver regra/política aplicada);
- `advisor_passport_id` e `advisor_proposal_cid` (quando houver proposta de advisor);
- `decider_identity` (quando houver decisão humana/NOC).

### 11.A.5 Não-destruição e supersedência
- Evidência supersedida MUST permanecer no ledger.
- Resultado de resolução MUST anotar `superseded_by` com ponteiro para o receipt de reconciliação.
- Qualquer auditoria legítima MUST conseguir reconstruir pré-conflito, conflito e pós-resolução.

### 11.A.6 Feedback para policy
- Conflitos recorrentes SHOULD acionar revisão de policy/WA.
- Resoluções manuais recorrentes SHOULD virar candidatas a automação.
- Política nova MUST ser versionada e MUST NOT reescrever retroativamente resoluções anteriores.

### 11.4 Função NOC
Homeostase não acontece só por código.
Precisa de operação humana com visão correta do sistema.

NOC no UBL deve:
- detectar desvios cedo,
- mostrar reconciliação entre fontes,
- priorizar intervenção por risco real,
- registrar ação corretiva como evidência.

### 11.5 Requisitos normativos (MUST)
- Plataforma MUST operar como nó de interoperabilidade e verificação coletiva.
- Plataforma MUST NOT invalidar soberania pessoal por conveniência operacional.
- Incidentes de consistência MUST gerar trilha auditável de diagnóstico e resposta.
- Política de rede MUST ser versionada e verificável.
- Observabilidade crítica MUST estar disponível sem exigir operação por terminal em fluxo normal.

### 11.6 Resultado esperado
Quando o sistema é pressionado, homeostase boa não esconde falha.
Ela contém dano, preserva prova e acelera retorno ao estado verificável.

---

## 12. Segurança, Capacidades e Limites

Segurança no UBL é propriedade de execução, não declaração de intenção.

### 12.1 Modelo de capacidade
Ações são permitidas por capacidade verificável em contexto:
- quem pede,
- sob qual identidade,
- em qual mundo,
- com qual escopo e limite.

Permissão é decisão runtime, não flag estática.

### 12.2 Limites como dignidade
Limite não é punição.
É proteção de integridade do sistema e do próprio ator.

Para humanos e LLMs:
- rights definem agência,
- duties definem responsabilidade,
- scope define fronteira,
- fuel define orçamento operacional.

### 12.3 Cadeia de confiança
A cadeia de confiança depende de:
- canon estável,
- assinatura rastreável,
- política executável,
- receipt verificável.

Quebrar qualquer elo degrada o todo.

### 12.4 Anti-bypass estrutural
O principal risco em sistemas complexos é “atalho legítimo”.
UBL trata isso como ameaça arquitetural.

Todo atalho de mutação fora do gate:
- quebra auditabilidade,
- quebra comparabilidade,
- quebra governança.

### 12.5 Requisitos normativos (MUST)
- Toda ação sensível MUST ser autenticada, autorizada e auditada.
- Verificação de capacidade MUST ocorrer no fluxo CHECK.
- Execução MUST respeitar limites de scope/fuel definidos em contrato.
- Chaves e identidades MUST ter rotação e trilha de proveniência.
- Falha de segurança MUST produzir evidência suficiente para análise forense.

### 12.6 Classes de Ameaça
O modelo de ameaça UBL prioriza:
- bypass de gate (mutação fora do pipeline),
- impersonação de contexto (pessoal se passando por plataforma ou vice-versa),
- adulteração de receipt (modificação pós-persistência),
- exaustão de fuel (negação de serviço por orçamento),
- colusão advisor-ator (advisor favorece interesse não declarado).

Cada classe MUST ter mitigação documentada e teste de contrato associado.

### 12.7 Princípio de desenho
No UBL, segurança boa não depende de operador perfeito.
Depende de arquitetura que torna o erro visível e o abuso caro.

## 13. Observabilidade e Operação (NOC)

Sem observabilidade, o protocolo vira crença.
Com observabilidade correta, ele vira instrumento.

### 13.1 Objetivo
A operação deve responder rapidamente:
- o que está saudável,
- o que está degradado,
- o que está inconsistente,
- o que exige intervenção humana agora.

### 13.2 Observabilidade orientada a evidência
Métrica isolada não basta.
Toda leitura operacional crítica deve apontar para prova:
- evento,
- receipt,
- artefato,
- reconciliação entre stores.

### 13.3 Visões mínimas do NOC
- saúde de pipeline por estágio,
- status de componentes e dependências,
- backlog/outbox/retries,
- reconciliação de auditoria,
- custos e latência de advisors,
- incidentes ativos e trilha de mitigação.

### 13.4 Operação sem terminal como padrão
Terminal é ferramenta válida, não requisito de sobrevivência.
Fluxos críticos de operação devem existir em interface humana robusta.

### 13.5 Requisitos normativos (MUST)
- NOC MUST expor estado real do sistema, não agregação enganosa.
- Alertas MUST ser acionáveis com vínculo para evidência.
- Ações operacionais MUST gerar trilha auditável.
- Degradação parcial MUST ser explicitada por componente e impacto.
- Runbooks MUST existir para falhas recorrentes de integridade, latência e reconciliação.

### 13.6 Resultado
Operação madura não elimina incidente.
Ela reduz surpresa e aumenta tempo de recuperação verificável.

---

## 14. Conformidade, Versionamento e Compatibilidade

Protocolos sobrevivem quando mudam sem perder identidade.

### 14.1 Conformidade
Conformidade no UBL não é checklist documental.
É teste executável contra invariantes constitucionais.

### 14.2 Invariantes de release
Toda release deve preservar:
- PF-01 (determinismo de conteúdo),
- PF-02 (receipt como evento),
- no-bypass,
- canon/CID lock,
- contrato de envelope e world.

### 14.3 Mudança e ruptura
Mudança breaking em canon, CID, envelope ou UNC exige:
- bump de versão explícito,
- plano de migração,
- estratégia de convivência temporária,
- critérios de rollback.

### 14.4 Compatibilidade operacional
Compatibilidade não é “funcionou localmente”.
É capacidade de dois nós em versões diferentes trocarem evidência sem ambiguidade semântica.

### 14.5 Requisitos normativos (MUST)
- Suites normativas MUST passar antes de release.
- Breaking changes MUST ter ADR e migração.
- Artefatos de conformance MUST ser preservados no ciclo de release.
- Compatibilidade MUST ser validada em cenário de interoperabilidade real.
- Release sem prova de conformidade MUST NOT ser tratada como estável.

### 14.6 Princípio
Velocidade sem compatibilidade quebra rede.
Compatibilidade sem evolução mata produto.
Versionamento é o pacto entre as duas.

---

## 15. Estado Atual (As-Built) e Lacunas Reais

Arquitetura viva precisa dizer a verdade sobre seu próprio estágio.

### 15.1 O que já está sólido
- pipeline canônico funcional,
- canon/CID operacionais,
- gate ativo,
- trilha de receipts e auditoria em evolução,
- fundamentos de AI Passport presentes no runtime.

### 15.2 Lacunas Críticas

| ID | Lacuna | Risco | Impacto | Dono | Critério de Pronto |
|----|--------|-------|---------|------|-------------------|
| L-01 | Advisory não referencia receipt em caminhos legados | Médio | Auditoria incompleta | Runtime | Advisory referencia receipt em 100% dos paths |
| L-02 | Passport enforcement parcial em ferramentas MCP | Alto | Agência sem identidade | Gate | Runtime rejeita advisory sem passport válido |
| L-03 | Narration auxiliar com decisão hardcoded | Médio | Diagnóstico enviesado | Advisory | Narration deriva decisão real do receipt |
| L-04 | Fronteiras fractais ainda não totalmente executáveis | Alto | Risco de colapso de soberania | Platform Engineering | Isolamento de chaves/tokens/stores/logs validado em testes de contrato |

### 15.3 Dívida explícita
Dívida técnica não deve ficar implícita em “TODO”.
Ela deve existir como backlog com:
- risco,
- impacto,
- dono,
- critério de pronto.

### 15.4 Requisitos normativos (MUST)
- Lacunas que afetam invariantes MUST ter prioridade de fechamento.
- Status “done” MUST corresponder a evidência de teste/execução.
- Workarounds temporários MUST ter prazo e condição de remoção.
- Documento de arquitetura MUST manter seção de lacunas atualizada por release.

### 15.5 Honestidade operacional
Sistema confiável não é sistema sem falha.
É sistema que sabe exatamente onde ainda pode falhar.

---

## 16. Roadmap de Fechamento

Roadmap no UBL não é wish list.
É sequência de redução de risco arquitetural.

### 16.1 Fase A — Fechar o núcleo
- consolidar invariantes em testes de contrato,
- eliminar bypass residual,
- fechar enforcement de identidade/capacidade.

### 16.2 Fase B — Fractal real
- extrair e estabilizar Core portátil,
- consolidar UBL Pessoal com advisor dedicado,
- separar definitivamente advisor pessoal vs plataforma em runtime.

### 16.3 Fase C — Plataforma estável
- fortalecer homeostase (auditoria unificada + operação NOC),
- endurecer governança de rede e compatibilidade inter-UBL,
- preparar trilho de produção com evidência de resiliência.

### 16.4 Fase D — Escala com integridade
- ampliar interoperabilidade sem diluir contrato canônico,
- evoluir ecossistema de componentes com segurança de supply chain,
- manter densidade de prova em todo aumento de superfície.

### 16.5 Critério de conclusão
Arquitetura v2 estará “fechada” quando:
- invariantes constitucionais estiverem cobertos por evidência executável,
- fronteiras fractais estiverem operacionais (não só textuais),
- operação diária conseguir manter saúde sem recorrer a exceções fora do protocolo.

### 16.6 Epílogo
UBL existe para sustentar agência com prova.

Nem máquina acima da pessoa.
Nem pessoa refém da máquina.
Um contrato em que ambos podem agir, responder e evoluir com rastro verificável.

---

## 17. Direções Futuras Priorizadas (Não Normativas)

Esta seção consolida propostas de evolução que agregam ao núcleo constitucional sem engessar a implementação.

### 17.1 Performance e escala de pipeline
- Execução paralela por `@world` SHOULD ser adotada para preservar serialização local e escalar horizontalmente.
- Bateladas com idempotência MAY ser usadas em cenários de alto throughput, com chave de idempotência obrigatória.
- Cache de decisão de policy MAY ser aplicado quando `chip_cid` e `policy_version` forem idênticos.

### 17.2 Hardening de fronteiras fractais
- Tokens de capacidade SHOULD carregar `context` criptograficamente vinculado ao emissor.
- Hierarquias de chave de contexto pessoal e plataforma SHOULD permanecer separadas por design e operação.
- Interações cross-context SHOULD ocorrer por handshake explícito via chip no gate, com token temporário de escopo estreito.

### 17.3 Integração advisor-runtime
- Tipo de chip de proposta (`proposal`) SHOULD ser formalizado para transformar intenção consultiva em entrada rastreável de pipeline.
- Saídas de advisor usadas em decisão humana SHOULD referenciar receipt de origem e gerar trilha própria.
- Feedback de resultado (proposal -> outcome) MAY ser usado para melhoria de advisor sob política explícita.

### 17.4 Reconciliação em escala federada
- Reconciliação por estruturas de hash (ex.: Merkle) MAY ser adotada para reduzir custo de comparação entre contextos.
- Protocolos de gossip MAY ser usados para detecção incremental de divergência entre nós federados.
- Provas de inclusão criptográfica SHOULD ser preferidas a transferência de histórico completo quando possível.

### 17.5 Decisão humana auditável
- Decisão humana em conflito SHOULD ser registrada como chip assinado pelo decisor e processada pelo pipeline completo.
- Casos de alto impacto MAY exigir política de múltiplas assinaturas (M-of-N).
- Toda ação humana de resolução SHOULD deixar trilha de custódia auditável.

### 17.6 Fuel e orçamento operacional
- Fuel multidimensional (compute, storage, network, advisor invocation) SHOULD evoluir como modelo padrão.
- Metrição em tempo real por estágio SHOULD produzir recibo de consumo para cada execução.
- Exaustão de fuel MUST permanecer erro explícito e auditável (sem bypass implícito).

### 17.7 Verificação de invariantes
- Modelagem formal (TLA+/Alloy) MAY ser adotada para validar propriedades globais do pipeline.
- Testes baseados em propriedades SHOULD complementar suites de conformidade.
- Invariantes críticos SHOULD ter asserts em builds de teste e validação contínua.

### 17.8 Interoperabilidade e padrões
- DID/VC MAY ser adotado como camada de interoperabilidade para identidade e passaporte, sem alterar invariantes do core.
- Alinhamento de CID e CAS com padrões amplamente usados SHOULD priorizar portabilidade.

### 17.9 Privacidade com ledger imutável
- Criptografia com destruição de chave SHOULD ser padrão para dados sensíveis sujeitos a remoção lógica.
- Tombstone chips SHOULD registrar solicitações de exclusão sem apagar trilha histórica.
- Consultas com controle de acesso e/ou provas de conhecimento zero MAY reduzir exposição de dados mantendo verificabilidade.

---

## Appendix A — Glossário

- **Chip** (§5): unidade atômica de intenção/ação no protocolo.
- **Receipt** (§9): prova de execução de um chip em um evento específico.
- **CID** (§5): identificador de conteúdo (`b3:` + BLAKE3 canônico).
- **Canon** (§5): representação determinística usada no trust path.
- **NRF-1.1** (§5): formato canônico de bytes do UBL.
- **Layer 0 / Layer 1** (§5): canon de máquina / JSON ancorado para autoria e leitura.
- **UNC-1** (§5): contrato canônico para números não-inteiros.
- **Gate** (§8): fronteira única de entrada de mutações.
- **Pipeline** (§4): KNOCK -> WA -> CHECK -> TR -> WF.
- **World** (§5): contexto de execução (`@world`).
- **Fuel** (§12): orçamento de execução.
- **Scope** (§6, §12): fronteira de agência permitida.
- **AI Passport** (§6): identidade operacional executável do advisor.
- **Advisor / Engine** (§7): camada consultiva de LLM, não soberana sobre decisões.
- **Homeostase** (§11): estabilidade operacional da rede sob mudança e conflito.
- **Reconciliação** (§9, §11): convergência auditável entre evidências de múltiplas fontes.
- **Soberania** (§3, §10): autonomia operacional de cada contexto pessoal/plataforma.
