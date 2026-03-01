# START-HERE

Este arquivo preserva os invariantes do pipeline e aponta para a documentação oficial.

## Invariantes que não podem quebrar

1. Pipeline canônico: `KNOCK -> WA -> CHECK -> TR -> WF`.
2. CID de chip é determinístico por conteúdo canônico.
3. Receipt CID é contextual (evento/runtime), não substitui CID de conteúdo.
4. Configuração central do Gate vem de `crates/ubl_config`.

## Navegação oficial

1. Leia `README.md`.
2. Vá para `docs/index.md`.
3. Consulte `docs/reference/config.md` para contrato de configuração.
4. Consulte `ops/gate/README.md` para execução operacional.
