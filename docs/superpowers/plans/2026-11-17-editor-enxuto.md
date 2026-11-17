# Editor enxuto — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deixar o editor mais limpo: lateral sem repetição, barra só com ícones, título fora da barra e menos vazio.

**Architecture:** Só mudanças de front-end (Vue). Uma função pura nova em `src/lib/outline.ts` decide quando um capítulo vira linha única. O resto é template e CSS em `Outline.vue`, `EditorScreen.vue` e `Typewriter.vue`.

**Tech Stack:** Vue 3 + TypeScript, Vitest, Tauri 2.

**Spec:** `docs/superpowers/specs/2026-11-17-editor-enxuto-design.md`

## Global Constraints
- Textos da interface em português do Brasil.
- Todo botão só com ícone tem `title` e `aria-label`.
- Nada some sem reaparecer no hover ou no foco do teclado (`:focus-visible`/`:focus-within`).
- Sem dependências novas.

## Review Focus
- Capítulo de linha única que é a cena atual: a linha tem que ficar marcada (borda vermelha), como hoje acontece com a cena.
- Renomear capítulo de linha única renomeia também a cena, para os dois continuarem iguais e a linha não "abrir" sozinha.
- Arrastar uma cena de outro capítulo para dentro de um capítulo de linha única: ele passa a ter 2 cenas e volta à lista normal.
- Capítulo vazio (0 cenas) continua mostrando o cabeçalho e aceitando cenas arrastadas.
- Usar só o teclado: Tab chega ao "+ Cena", à lixeira e aos ícones da barra, e todos ficam visíveis no foco.

---

### Task 1: Regra da linha única

**Files:**
- Modify: `src/lib/outline.ts`
- Test: `src/lib/outline.test.ts`

**Interfaces:**
- Produces: `singleScene(c: Outline["chapters"][number]): SceneSummary | null` — retorna a única cena quando o capítulo tem exatamente 1 cena e `scene.title.trim() === chapter.title.trim()`; senão `null`.

- [ ] **Step 1: Testes** em `describe("singleScene")`:
  - 1 cena com mesmo título → retorna a cena
  - 1 cena com título diferente → `null`
  - 2 cenas → `null`
  - 0 cenas → `null`
  - títulos iguais com espaço sobrando (`" X "` vs `"X"`) → retorna a cena
- [ ] **Step 2:** `npx vitest run src/lib/outline.test.ts` → FAIL (função não existe)
- [ ] **Step 3:** Implementar `singleScene` em `src/lib/outline.ts`.
- [ ] **Step 4:** `npx vitest run src/lib/outline.test.ts` → PASS
- [ ] **Step 5:** Commit `feat(sumario): regra de capítulo de cena única`

### Task 2: Lateral enxuta

**Files:**
- Modify: `src/features/editor/Outline.vue`

**Interfaces:**
- Consumes: `singleScene` (Task 1), `sceneRename(id, title)` de `src/lib/api.ts`.

- [ ] **Step 1: Linha única.** Quando `singleScene(c)` não é `null`, o cabeçalho do capítulo vira um botão que emite `select` com o id da cena. Ele recebe a classe `current` (borda esquerda `var(--ribbon)`, mesmo visual de `.scene.current .scene-btn`) quando for a cena atual. A `<ol>` de cenas continua no DOM como zona de arrastar, mas o `<li>` dessa cena fica escondido (`v-show`/classe). Assim, soltar outra cena ali continua funcionando.
- [ ] **Step 2: Renomear.** O duplo clique no cabeçalho continua renomeando o capítulo. Em linha única, `finishRename` também chama `sceneRename(cena.id, t)`. Cenas comuns ganham o duplo clique para renomear, com o mesmo input de `rename`, chamando `sceneRename`. O `editing` guarda o id e funciona para os dois.
- [ ] **Step 3: "+ Cena" no hover.** O `.add` dentro do capítulo fica com `opacity: 0` e aparece com `.chapter:hover .add, .add:focus-visible { opacity: 1 }`. Para não deixar buraco, reduza a margem dele. O botão "Novo capítulo" não muda.
- [ ] **Step 4: Topo sem corte.** O `header` fixo da lateral precisa cobrir por inteiro o que rola por baixo. Ele fica com fundo opaco, sem margem que deixe frestas, e com `box-shadow: 0 6px 8px -6px rgba(0,0,0,.5)`. Se o header não existir mais em `Outline.vue` (o título do livro foi para a barra), aplique a correção ao `padding-top` do `.outline`, ou use `mask-image` com um degradê curto no topo.
- [ ] **Step 5: Verificar** com `npm run build` (sem erro de tipo) e `npm run dev`, abrindo um livro importado de EPUB. Cada capítulo deve aparecer como uma linha só, sem nome repetido.
- [ ] **Step 6:** Commit `feat(sumario): capítulo de cena única em uma linha e "+ Cena" no hover`

### Task 3: Barra só com ícones e título fora dela

**Files:**
- Modify: `src/features/editor/EditorScreen.vue` (template da `.bar`, linhas ~272-297; CSS ~396-422)

- [ ] **Step 1:** Remover o `<input class="scene-title">`, a função `renameScene` e o CSS `.scene-title`, se nada mais usar. O espaço livre vai para um `<span class="spacer" style="flex:1">` antes do status.
- [ ] **Step 2:** "Livros", Ideias, Versões, Exportar e Ajustes viram só ícone. O texto vai para `title` (com o atalho, como hoje) e `aria-label`. O botão "Livros" fica com `title="Voltar à biblioteca"` e `aria-label="Voltar à biblioteca"`. O `.badge` de ideias continua.
- [ ] **Step 3: Verificar** com `npm run build` e `npm run dev`. A barra deve mostrar só seta, sumário, status e ícones. O título aparece apenas no topo da folha, e passar o mouse em cada ícone mostra o nome.
- [ ] **Step 4:** Commit `feat(editor): barra só com ícones; título da cena só na folha`

### Task 4: Menos vazio no fim da cena

**Files:**
- Modify: `src/features/editor/Typewriter.vue:102` e `:27`

- [ ] **Step 1:** Em `.page`, o padding de baixo vira `34vh`. Em `centerCaret`, o alvo passa de `rect.height * 0.5` para `rect.height * 0.66`, para a linha do cursor ficar um pouco abaixo do meio. Assim, a última linha chega à mesma altura com menos folga.
- [ ] **Step 2: Verificar** com `npm run dev`. Ao digitar no fim da cena, a linha fica estável e sem pulo. Abaixo do rodapé "N palavras nesta cena" sobra cerca de um terço da tela.
- [ ] **Step 3:** Commit `style(editor): menos folga no fim da cena`

### Task 5: Fechamento

- [ ] `npm test` → todos passam.
- [ ] `npx tauri build` → gera `Lume.app` e `.dmg`.
