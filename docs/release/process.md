# Release Process — UBL-CORE

## Versionamento

- SemVer para crates e artefatos publicados.

## Como cortar release

1. Garantir quality gate local/CI.
2. Atualizar changelog relevante.
3. Criar tag `v*`.
4. Executar promoção conforme fluxo de release do repositório.

## Changelog

- Registrar mudanças de comportamento, compat e operações.

## Rollout / rollback

- Rollout gradual (canário quando aplicável).
- Rollback por reversão de tag/imagem e restauração de config estável.

## Compat policy

### Compat/shim deprecated
- `rb_vm` e `ubl_ai_nrf1` são shims temporários.
- Código novo deve usar `ubl_vm` e `ubl_nrf`.
- Toda remoção de shim exige aviso prévio e janela de migração.
