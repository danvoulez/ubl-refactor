# ADR-0001: Keep legacy shims for suite compatibility

- Data: 2026-03-01
- Status: Accepted

## Contexto
Após a extração de `ubl_vm` e `ubl_nrf` como crates reais, ainda existe dependência de nomes antigos em parte da suíte e integrações.

## Decisão
Manter os shims legados como compatibilidade temporária.

### Compat/shim deprecated
- `ubl_vm` e `ubl_nrf` são legados.
- Novo código deve depender de `ubl_vm` e `ubl_nrf`.
- Remoção dos shims depende de migração completa da suíte e consumidores.

## Consequências
- Reduz risco de quebra em CI e ambientes externos.
- Mantém débito técnico explícito e rastreável.

## Alternativas consideradas
- Remoção imediata dos shims (rejeitada por risco de compatibilidade).
