# Threat Model — UBL Gate

## Escopo

Serviço Gate HTTP, pipeline determinístico e integrações de storage/config.

## Trust boundaries

- Cliente externo → API HTTP do Gate.
- Gate → storage/eventstore/outbox.
- Gate → runtime/policy execution.

## Riscos principais e mitigação

1. **Auth/Authz inadequada**
   - Mitigação: validação estrita de política e controles de acesso no perímetro.
2. **Replay**
   - Mitigação: nonce/idempotência e checagens de conflito.
3. **Poison payload / input malicioso**
   - Mitigação: validação KNOCK, canon determinístico e taxonomy de erro.
4. **DoS / exaustão de recursos**
   - Mitigação: limites de payload/recursos, observabilidade e rate controls.
5. **Storage corruption / inconsistência**
   - Mitigação: backend confiável, backups, validação de integridade e rollback operacional.

## Fora de escopo

- Ameaças de infraestrutura externa não gerenciadas por este repositório.
- Segurança de sistemas cliente fora da fronteira do Gate.
