# Local Source Trust Flow (Gitea + MinIO + ublx + PM2)

## Status

- Status: Active
- Owner: Ops + Security
- Scope: Operacao continua de source/distribuicao/deploy
- Out of scope: `forever_bootstrap` (fluxo de bootstrap inicial)

## Objetivo

Definir o padrao oficial para:

1. manter o repositorio oficial em Gitea local (nosso "GitHub local"),
2. espelhar para GitHub real quando o perfil exigir,
3. publicar bundles assinados em MinIO/S3 versionado,
4. emitir recibos operacionais via `ublx submit`,
5. promover deploy de binarios com reload em PM2,
6. padronizar nome e metadados dos artefatos com as mesmas ancoras do UBL (`app/tenant/user/world`).

## Arquitetura Operacional

1. Git source of truth: Gitea (`gitea.logline.world`, remote `gitea`)
2. Mirror publico: GitHub (`origin`)
3. Storage assinado/versionado: MinIO bucket `ubl-source`
4. Runtime de recibos: UBL Gate (`SOURCE_GATE_URL`, default `http://127.0.0.1:4000`)
5. Orquestracao de processos: PM2 (apps como `gitea`, `minio`, `ubl-gate`)

## Configuracao Canonica

Arquivo: `ops/source_flow.env`

Campos chave:

- `SOURCE_PROFILE=public_core`
- `SOURCE_GITEA_REMOTE_NAME=gitea`
- `SOURCE_GITEA_URL=git@gitea.logline.world:foundation/UBL-CORE.git`
- `SOURCE_GITHUB_REMOTE_NAME=origin`
- `SOURCE_PUBLIC_REQUIRE_GITHUB_MIRROR=true`
- `SOURCE_S3_ENDPOINT=http://127.0.0.1:9000`
- `SOURCE_S3_BUCKET=ubl-source`
- `SOURCE_UBLX_BIN=/Users/ubl-ops/.local/bin/ublx`
- `SOURCE_UBLX_CARGO_FALLBACK=true`
- `SOURCE_EMIT_CHIP=true`

## Verdade Oficial de Codigo

No repositorio local, os remotes oficiais sao:

- `gitea` -> `git@gitea.logline.world:foundation/UBL-CORE.git`
- `origin` -> `https://github.com/LogLine-Foundation/UBL-CORE`

Politica:

1. O source de trabalho e publicado em `gitea`.
2. Em `public_core`, `publish` so passa se GitHub estiver no mesmo HEAD.
3. O artefato versionado/assinado em MinIO vira a unidade de distribuicao.

## Comandos Oficiais

Entrada principal:

```bash
scripts/ubl_ops.sh source-flow <command> --env ops/source_flow.env
```

Comandos principais:

```bash
# perfil e gates ativos
scripts/ubl_ops.sh source-flow profile --env ops/source_flow.env

# configurar remote gitea + pushDefault
scripts/ubl_ops.sh source-flow init-remote --env ops/source_flow.env

# push HEAD para gitea (source of truth)
scripts/ubl_ops.sh source-flow push --env ops/source_flow.env --branch main

# espelhar HEAD para github real (quando exigido)
scripts/ubl_ops.sh source-flow mirror-github --env ops/source_flow.env --branch main

# gerar bundle assinado, enviar para MinIO e emitir chip ops/source.publish.v1
scripts/ubl_ops.sh source-flow publish --env ops/source_flow.env

# baixar bundle, promover release, opcional build/reload, emitir ops/source.deploy.v1
scripts/ubl_ops.sh source-flow deploy --env ops/source_flow.env
```

## Regra de Seguranca: GitHub Real no Mesmo HEAD

No perfil `public_core`, o `publish` valida:

1. branch local atual,
2. SHA local,
3. SHA remoto em `origin/<branch>`.

Se divergir, falha fechada e exige `mirror-github` antes de publicar.

## Recibos via `ublx`

`source_flow.sh` resolve o CLI nesta ordem:

1. `SOURCE_UBLX_BIN` (padrao recomendado: `/Users/ubl-ops/.local/bin/ublx`)
2. fallback: `cargo run -p ubl_cli -- submit` se `SOURCE_UBLX_CARGO_FALLBACK=true`

Observacao importante:

- O binario em `~/.local/bin/ublx` precisa expor `submit`.
- O `ublx` em `~/.cargo/bin` pode estar em versao diferente.

## Padronizacao de Cache Node/Cargo

O fluxo oficial inclui cache compartilhado lock-keyed para reduzir build time:

```bash
scripts/ubl_ops.sh source-flow cache-profile --env ops/source_flow.env
scripts/ubl_ops.sh source-flow cache-save --env ops/source_flow.env
scripts/ubl_ops.sh source-flow cache-restore --env ops/source_flow.env
```

Saida de perfil:

- `ops/source_cache.env` com:
  - `NPM_CONFIG_CACHE`
  - `PNPM_STORE_PATH`
  - `CARGO_HOME`
  - `CARGO_TARGET_DIR`

Detalhes:

1. `node_modules` e opcional (`SOURCE_CACHE_INCLUDE_NODE_MODULES`).
2. Cache Rust usa hash de `Cargo.lock`.
3. Cache JS usa hash do lockfile JS presente no repo.

## Artefatos e Binarios para PM2

`deploy` promove para:

- `${SOURCE_DEPLOY_ROOT}/releases/<commit>`
- symlink `${SOURCE_DEPLOY_ROOT}/current`

Hooks opcionais no `ops/source_flow.env`:

- `SOURCE_DEPLOY_BUILD_CMD` (ex.: build de binario)
- `SOURCE_PM2_RELOAD_CMD` (ex.: reload dos apps PM2)

Cadeia recomendada:

1. Source validado e publicado (Gitea/GitHub/MinIO),
2. Bundle assinado puxado no destino,
3. Build deterministico do binario,
4. Reload controlado via PM2,
5. Recibo de deploy emitido no gate.

## Acesso MinIO (Desktop e iPhone)

### Credenciais

Arquivo local:

- `/Users/ubl-ops/.secrets/minio.env`

Campos:

- `MINIO_ROOT_USER`
- `MINIO_ROOT_PASSWORD`

Recomendacao de qualidade:

1. Em producao, preferir service account no lugar de root.
2. Segredo nunca em repo.
3. Rotacao periodica de chaves.

### Endpoint correto

- API MinIO: `http://<HOST>:9000`
- Console MinIO: `http://<HOST>:9001`

No iPhone:

1. usar IP do Mac na rede local (`192.168.x.x`),
2. nao usar `127.0.0.1`/`localhost`.

## Padrao Unico de Nome + Metadados (alinhado ao UBL world)

### Ancora canonica

Usar o mesmo eixo de mundo do UBL:

- `@world = a/{app}/t/{tenant}`

Tags obrigatorias de identidade:

- `ubl/app=<app>`
- `ubl/tenant=<tenant>`
- `ubl/user=<user|system>`
- `ubl/world=a/<app>/t/<tenant>`

### Chave de objeto MinIO (object key)

```text
a/<app>/t/<tenant>/artifacts/<artifact_type>/<yyyy>/<mm>/<dd>/<id>.v<major>.<ext>
```

Exemplo:

```text
a/logline/t/main/artifacts/source.bundle/2026/02/23/source-53b43ced.v1.tar.gz
```

### Naming obrigatório

1. minusčulas apenas,
2. sem espaços,
3. sem acentos,
4. versao explicita (`v1`, `v2`),
5. data em ISO no metadata (`created_at`).

### Metadata mínima de objeto

- `ubl/type=<artifact_type>`
- `ubl/ver=v1`
- `ubl/env=<dev|stg|prod>`
- `ubl/commit=<git_sha>`
- `ubl/cid=<b3:...>` (quando houver)
- `ubl/created_at=<ISO8601>`

### Envelope JSON interno recomendado

```json
{
  "@type": "ubl/artifact",
  "@id": "source-bundle-2026-02-23-2209z",
  "@ver": "1.0",
  "@world": "a/logline/t/main",
  "artifact_type": "source.bundle",
  "source_commit": "53b43ced5e6836f8d0acaf4f22db433ffd434998",
  "created_at": "2026-02-23T22:09:45Z"
}
```

## Qualidade (Checklist Go/No-Go)

1. Gitea online e acessivel (`:3301`, `:2222`).
2. MinIO online (`:9000`, `:9001`).
3. Bucket `ubl-source` existente e com versionamento ativo.
4. `ublx submit` disponivel no binario configurado.
5. `publish` gerando `manifest.json`, `manifest.sig`, `source.bundle`, `latest.json`.
6. `deploy` gerando release + symlink current + recibo de deploy.
7. Metadados obrigatorios (`ubl/app`, `ubl/tenant`, `ubl/user`, `ubl/world`) presentes nos objetos novos.

## Estado Verificado (host local)

Checklist de evidencia observado:

1. PM2 online para `gitea`, `minio` e `ubl-gate`.
2. Remotes do repo configurados em `gitea` + `origin`.
3. Bucket `ubl-source` com `Status: Enabled` em versionamento.
4. Prefixo de source publish presente em `repos/logline/UBL-CORE/<commit>/...`.
5. `~/.local/bin/ublx` com comando `submit` disponivel.

## Runbook Curto

```bash
cd /Users/ubl-ops/UBL-CORE

# 1) validar perfil
scripts/ubl_ops.sh source-flow profile --env ops/source_flow.env

# 2) push no source of truth local
scripts/ubl_ops.sh source-flow push --env ops/source_flow.env --branch main

# 3) mirror no github real (public_core)
scripts/ubl_ops.sh source-flow mirror-github --env ops/source_flow.env --branch main

# 4) publicar bundle assinado + recibo
scripts/ubl_ops.sh source-flow publish --env ops/source_flow.env

# 5) deploy + reload + recibo
scripts/ubl_ops.sh source-flow deploy --env ops/source_flow.env
```

## Referencias

- `docs/ops/GITEA_SOURCE_FLOW.md`
- `scripts/source_flow.sh`
- `scripts/ubl_ops.sh`
- `ops/source_flow.env`
- `docs/canon/CANON-REFERENCE.md` (`@world` anchors)
