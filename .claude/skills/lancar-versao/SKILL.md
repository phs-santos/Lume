---
name: lancar-versao
description: Lança uma versão nova do Lume de ponta a ponta — junta o que foi feito, escreve as notas, cria a versão, gera os instaladores no GitHub e publica. Use quando pedirem para lançar, publicar ou soltar uma versão nova (ex. "/lancar-versao minor", "solta uma versão de correção").
---

# Lançar uma versão do Lume

Argumentos (opcionais, em qualquer ordem):
- tipo: `patch` (só correções), `minor` (coisa nova), `major` (mudança grande) ou versão exata `X.Y.Z`. Sem tipo: escolha pelo que entrou — `minor` se houver qualquer `feat`, senão `patch`.
- `rascunho`: deixa o release em rascunho em vez de publicar.

## Passos

1. **Nada para lançar?** `git log $(git describe --tags --abbrev=0 --match 'v*')..HEAD --oneline`. Vazio: diga que não há nada novo desde a última versão e pare.
2. **Calcule a versão nova** sem gravar: leia a versão atual em `src-tauri/tauri.conf.json` e aplique o tipo (`patch` soma 1 no último número; `minor` soma no do meio e zera o último; `major` soma no primeiro e zera os outros).
3. **Notas:** chame o agente `notas-da-versao` com a versão nova. Ele reescreve o topo de `.github/release-notes.md`.
4. **Mostre as notas** ao usuário (a seção "O que mudou") numa mensagem curta. Se ele estiver acompanhando e pedir ajuste, ajuste antes de seguir; se não houver resposta, siga.
5. **Lançar:** chame o agente `lancador` com o tipo e `publicar = sim` (ou `rascunho`). Ele roda os testes, sobe a versão, cria a tag, acompanha o build (25 a 40 min), repete os builds do Mac que ficarem sem máquina, confere os arquivos e publica.
6. **Relate** em poucas linhas: versão, link do release, publicado ou rascunho, e o comando de instalação no Mac:

   ```bash
   curl -fsSL https://raw.githubusercontent.com/phs-santos/Lume/main/scripts/install-macos.sh | bash
   ```

Se um agente parar com erro (teste falhando, build quebrado, arquivo faltando), mostre o erro ao usuário e não tente contornar: nada de pular testes, forçar push ou apagar tag.
