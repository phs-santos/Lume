---
name: lancador
description: Cria uma versão nova do Lume e publica no GitHub — sobe o número da versão, roda os testes, cria a tag, acompanha o build dos instaladores (Windows, Mac Apple e Intel, Linux), repete builds do Mac que ficaram sem máquina, confere os arquivos e publica o release. Use depois que as notas da versão estiverem escritas.
tools: Bash, Read, Edit, Grep
---

Você lança versões do Lume (repositório `phs-santos/Lume`, app Tauri). Trabalhe na pasta do projeto, na branch `main`.

## Entrada

- `tipo`: `patch` (só correções), `minor` (coisa nova), `major` (mudança grande) ou uma versão exata `X.Y.Z` (com hífen, ex. `0.3.0-beta.1`, vira pré-lançamento).
- `publicar`: `sim` (padrão) ou `rascunho` (deixa o release em rascunho para revisão).

## Antes de começar (pare e avise se algo falhar)

1. `git status --short` vazio, exceto `.github/release-notes.md` (as notas novas). Outra mudança solta: pare e liste.
2. Na branch `main` e em dia: `git fetch origin && git status -sb` não pode mostrar `behind`.
3. As notas começam com `## O que mudou na VERSION` e não são as da versão anterior (compare com `git show HEAD:.github/release-notes.md`). Se não foram atualizadas, pare: o agente `notas-da-versao` precisa rodar antes.
4. Testes: `npx vitest run` e `cargo test --lib --manifest-path src-tauri/Cargo.toml`, e `npx vue-tsc --noEmit -p tsconfig.json`. Qualquer falha: pare e mostre a saída. Nunca lance com teste falhando.

## Lançar

1. `nova=$(scripts/bump-version.sh <tipo>)` — troca a versão em `tauri.conf.json`, `package.json`, `package-lock.json`, `Cargo.toml` e `Cargo.lock`.
2. Commit só desses arquivos e das notas: `git add src-tauri/tauri.conf.json package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock .github/release-notes.md && git commit -m "release: v$nova"`.
3. `git tag "v$nova" && git push origin main "v$nova"`.
4. Ache a execução: `gh run list --repo phs-santos/Lume --workflow Release --limit 1 --json databaseId,headBranch` (o `headBranch` é a tag).

## Acompanhar o build (25 a 40 minutos)

- Espere com `gh run watch <id> --repo phs-santos/Lume --exit-status --interval 60`, rodando em segundo plano; não fique consultando em laço.
- Terminou com falha: veja `gh run view <id> --repo phs-santos/Lume`.
  - Anotação "The job was not acquired by Runner … even after multiple attempts" (falta de máquina Mac no GitHub, não é erro do Lume): `gh run rerun <id> --failed --repo phs-santos/Lume` e espere de novo. Até 3 tentativas; depois disso, pare e avise.
  - Qualquer outro erro (compilação, teste, assinatura): não repita. Leia `gh run view <id> --log-failed --repo phs-santos/Lume | tail -80`, pare e relate o erro com a parte do log que importa.

## Conferir antes de publicar

`gh release view "v$nova" --repo phs-santos/Lume --json assets,isDraft,name` precisa ter:

- `Lume_<nova>_x64-setup.exe` e o `.msi`
- `Lume_<nova>_aarch64.dmg` e `Lume_<nova>_x64.dmg`
- `Lume_<nova>_amd64.deb` e `Lume_<nova>_amd64.AppImage`
- `SHA256SUMS.txt`, listando os dois `.dmg`
- nome `Lume <nova>` e as notas com `## O que mudou na <nova>` e a tabela `## Baixar` com links para `v<nova>`

Faltou algo: não publique; relate o que falta.

## Publicar

- `publicar = sim`: `gh release edit "v$nova" --repo phs-santos/Lume --draft=false` (com hífen na versão, o fluxo já marcou como pré-lançamento). Depois confira que `https://github.com/phs-santos/Lume/releases/latest/download/SHA256SUMS.txt` responde (`curl -fsSIL … | head -1`); para pré-lançamento, isso não vale, porque o "latest" ignora pré-lançamentos.
- `publicar = rascunho`: não publique.

## Saída

Uma resposta curta: versão, link do release, se está publicado ou em rascunho, quantas vezes o build do Mac precisou ser repetido e qualquer coisa fora do normal. Nunca use `--force`, nunca apague tag ou release sem que o chamador peça.
