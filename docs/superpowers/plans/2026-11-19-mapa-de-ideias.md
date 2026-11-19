# Criação de livro com IA e mapa de ideias — plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Nome provisório e "desenvolver a ideia com a IA" na criação do livro, e um mapa de cartões ligados por setas em cada livro, onde a IA propõe cartões que a pessoa aprova.

**Architecture:** Dados novos no SQLCipher (migração 6, módulo `db/map.rs`), expostos por comandos `db_cmd!` e por `src/lib/api.ts`. A IA ganha ferramentas `ler_mapa`, `propor_cartoes` e `propor_nome`; as propostas viram `ChatEvent::MapProposal` e um bloco na conversa. O mapa é um componente Vue (`src/features/map/`) sobre Vue Flow, numa aba Texto | Mapa do editor.

**Tech Stack:** Tauri 2 (Rust, rusqlite/SQLCipher), Vue 3 + TS, `@vue-flow/core` ^1.48.2, Vitest + happy-dom, `cargo test`.

**Spec:** `docs/superpowers/specs/2026-11-19-mapa-de-ideias-design.md`

## Global Constraints

- Nome vazio (só espaços conta como vazio) → título `Livro sem nome` e `title_provisional = 1`. `book_rename` desliga o provisório. Aceitar `propor_nome` desliga o provisório.
- Conversa de ideia: `purpose = 'ideia'`, título `Desenvolvendo a ideia`.
- Cores de cartão, nesta ordem: `papel`, `amarelo`, `verde`, `azul`, `rosa` (padrão `papel`), definidas como tokens em `src/styles/themes.css` (claro e escuro).
- A IA nunca altera mapa, nome ou texto sem aprovação. `propor_cartoes` existe nos dois modos; `propor_nome` só em conversa de ideia.
- No contexto da IA, "Mapa do livro" (o sumário) passa a se chamar "Sumário do livro", inclusive na descrição de `ler_cena`.
- Nada do mapa sai do cofre; a chave da IA nunca vai para a interface.
- Textos de interface em português do Brasil, sem jargão. Dica do mapa vazio, exata: "Dê dois cliques para criar um cartão. Puxe a bolinha de um cartão até outro para ligar."
- O mapa só carrega quando a aba Mapa é aberta (import dinâmico do componente).
- Todo comando Rust novo: registrar em `src-tauri/src/lib.rs` e espelhar em `src/dev/mockBackend.ts` (o mock do navegador).

## Review Focus

1. A IA propõe setas para ids que não existem ou que são de outro livro → a ferramenta devolve erro ao modelo e não grava nada (teste no Task 3).
2. A pessoa apaga um cartão citado numa proposta pendente e depois aceita → a seta órfã é ignorada; o resto entra (teste no Task 3).
3. Clique duplo em "Pôr no mapa" → a segunda aceitação falha sem duplicar cartões (teste no Task 3).
4. "Transformar em ficha" duas vezes no mesmo cartão → devolve a mesma ficha, sem criar outra (teste no Task 1).
5. Nome só com espaços → `Livro sem nome` provisório; importar EPUB continua com o título do arquivo, não provisório (teste no Task 1).

---

### Task 1: Dados do mapa, nome provisório e propósito da conversa

**Files:**
- Modify: `src-tauri/src/db/mod.rs` (6ª entrada em `MIGRATIONS`, `pub mod map;`, teste de migração)
- Create: `src-tauri/src/db/map.rs`
- Modify: `src-tauri/src/db/books.rs` (`Book.title_provisional`, `book_create`, `book_rename`, `book_row` e os SELECTs)
- Modify: `src-tauri/src/db/ai.rs` (`Conversation.purpose`, `CONV_COLS`, `conversation_new_idea`)

**Interfaces:**
- Produces (Rust, `db::map`):
  - `struct MapCard { id, book_id, text: String, color: String, x: f64, y: f64, bible_entry_id: Option<String>, created_at: i64, updated_at: i64 }` (Serialize)
  - `struct MapLink { id, book_id, from_card, to_card, label: String }`
  - `struct MapData { cards: Vec<MapCard>, links: Vec<MapLink> }`
  - `fn map_get(c, book_id) -> Result<MapData>`
  - `fn card_create(c, book_id, text, color, x, y, now) -> Result<MapCard>` (cor fora da lista → `DbError::Invalid`)
  - `fn card_update(c, id, text, color, now) -> Result<MapCard>`
  - `fn card_move(c, id, x, y, now) -> Result<()>`
  - `fn card_delete(c, id) -> Result<()>` (setas caem por CASCADE)
  - `fn link_create(c, book_id, from, to) -> Result<MapLink>` (os dois cartões precisam ser do livro; `from == to` → Invalid; seta repetida devolve a existente)
  - `fn link_set_label(c, id, label) -> Result<()>`, `fn link_delete(c, id) -> Result<()>`
  - `fn card_to_bible(c, card_id, kind, now) -> Result<BibleEntry>`: 1ª linha = nome, resto = corpo; `edited_by_user = 1`; grava `bible_entry_id`; se já tiver ficha existente, devolve essa.
  - `const COLORS: [&str; 5]`
- Produces: `Book.title_provisional: bool`; `Conversation.purpose: String`; `ai::conversation_new_idea(c, book_id, now) -> Result<Conversation>` (fecha as outras, cria com purpose `ideia` e título `Desenvolvendo a ideia`).
- Migração 6: o SQL da seção "Dados" do spec (`map_proposals` incluído; usado no Task 3), mais `CREATE INDEX map_cards_book ON map_cards(book_id)`.

- [ ] **Step 1: Testes que falham** em `db/map.rs`, `db/books.rs`, `db/ai.rs`, `db/mod.rs`:
  - `migration_v6_runs_on_a_v5_database` (molde de `migration_v5_runs_on_a_v4_database`; livro existente fica com `title_provisional = 0`, conversa com `purpose = 'livro'`)
  - `blank_title_becomes_provisional`: `book_create(c, "   ", "conselheira", t)` → título `Livro sem nome`, `title_provisional == true`; `book_rename` → `false`
  - `named_book_is_not_provisional`
  - `cards_and_links_roundtrip`: cria 2 cartões, liga, `map_get` devolve 2 + 1; `card_delete` do primeiro apaga a seta
  - `link_rejects_card_from_other_book` e `link_rejects_self`
  - `invalid_color_rejected`
  - `card_to_bible_splits_first_line_and_is_idempotent`: texto `"Marina\nFaroleira, 30 anos"` → ficha `personagem` com nome `Marina` e corpo `Faroleira, 30 anos`; segunda chamada devolve o mesmo id e `bible_list` tem 1
  - `deleting_bible_entry_clears_card_link`
  - `conversation_new_idea_has_purpose_and_title`
- [ ] **Step 2:** `cargo test --lib --manifest-path src-tauri/Cargo.toml db::` → falham (não compila).
- [ ] **Step 3:** Implemente as interfaces acima.
- [ ] **Step 4:** `cargo test --lib --manifest-path src-tauri/Cargo.toml` → tudo passa (inclusive o import de EPUB, que continua não provisório).
- [ ] **Step 5:** Commit `feat(db): mapa de ideias, nome provisório e conversa de ideia`.

### Task 2: Comandos e API do mapa

**Files:**
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`
- Modify: `src/lib/types.ts`, `src/lib/api.ts`, `src/dev/mockBackend.ts`

**Interfaces:**
- Consumes: Task 1.
- Produces (comandos `db_cmd!`; nomes TS em camelCase):
  - `map_get(book_id) -> MapData` → `mapGet(bookId)`
  - `map_card_create(book_id, text, color, x, y)`, `map_card_update(id, text, color)`, `map_card_move(id, x, y)`, `map_card_delete(id)`
  - `map_link_create(book_id, from, to)`, `map_link_label(id, label)`, `map_link_delete(id)`
  - `map_card_to_bible(id, kind) -> BibleEntry`
  - `ai_conversation_new_idea(book_id) -> ConversationView` → `aiConversationNewIdea(bookId)`
  - TS: `Book.title_provisional: boolean`, `Conversation.purpose: "livro" | "ideia"`, `MapCard`, `MapLink`, `MapData`, `type CardColor = "papel" | "amarelo" | "verde" | "azul" | "rosa"`.
- O mock imita as regras do Task 1 que a interface percebe (nome vazio vira provisório; ficha idempotente).

- [ ] **Step 1:** Implemente os comandos, o registro e os wrappers.
- [ ] **Step 2:** `cargo build --manifest-path src-tauri/Cargo.toml` e `npx vue-tsc --noEmit -p tsconfig.json` sem erros; `npx vitest run` continua passando.
- [ ] **Step 3:** Commit `feat: comandos do mapa de ideias`.

### Task 3: A IA lê o mapa e propõe cartões e nome

**Files:**
- Modify: `src-tauri/src/db/map.rs` (propostas)
- Modify: `src-tauri/src/ai/tools.rs`, `src-tauri/src/ai/chat.rs`, `src-tauri/src/ai/context.rs`
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src/lib/types.ts`, `src/lib/api.ts`, `src/dev/mockBackend.ts`

**Interfaces:**
- Produces (`db::map`):
  - `struct MapProposal { id, book_id, conversation_id: Option<String>, kind: String, payload_json: String, status: String, created_at: i64 }`
  - Payload `cartoes`: `{"cartoes":[{"ref":"a","texto":"…"}],"setas":[{"de":"a","para":"<ref ou id de cartão>","rotulo":"…"}]}`. Payload `nome`: `{"nome":"…"}`.
  - `fn proposal_add_cards(c, book_id, conv_id, payload: &Value, now) -> Result<MapProposal>`: valida 1 a 12 cartões, texto não vazio (até 500 caracteres), refs únicos, e cada `de`/`para` é um ref novo ou um cartão deste livro; senão `Invalid` com a razão.
  - `fn proposal_add_name(c, book_id, conv_id, nome, now) -> Result<MapProposal>`
  - `fn proposals_pending(c, book_id) -> Result<Vec<MapProposal>>`
  - `fn proposal_accept(c, id, refs: &[String], now) -> Result<Vec<MapCard>>`: só `pendente` (senão `Invalid("proposta já resolvida")`). Para `cartoes`: cria os cartões dos `refs` escolhidos (vazio → Invalid) na grade abaixo; cria as setas cujos dois lados existem (ref aceito ou cartão ainda existente) e ignora as outras; status `aceita`. Para `nome`: `book_rename` (desliga o provisório) e devolve `[]`. Tudo numa transação.
  - `fn proposal_discard(c, id) -> Result<()>`
  - Grade: origem `x = max(x) + 320` e `y = min(y)` dos cartões existentes (sem cartões: 0, 0); 3 colunas, passo 280 na horizontal e 180 na vertical, na ordem da proposta.
- Produces (IA):
  - `ToolOutcome` ganha `map_proposal: Option<MapProposal>`; `ChatEvent::MapProposal { proposal: MapProposal }` (TS: `{ type: "map_proposal"; proposal: MapProposal }`).
  - `tools::defs(mode, purpose)`: `ler_mapa` e `propor_cartoes` sempre; `propor_nome` (`{nome}`) só com purpose `ideia`. `run` recebe `purpose` e recusa `propor_nome` fora da ideia.
  - `ler_mapa`: uma linha por cartão `- [id] texto` (texto em uma linha, até 200 caracteres) e uma por seta `id → id: rótulo`; mapa vazio → `(mapa vazio)`; nota `consultei o mapa`.
  - `context::build`: renomeia para "Sumário do livro"; com purpose `ideia`, acrescenta instruções: desenvolver a ideia da pessoa, uma pergunta por vez (premissa, protagonista, o que quer, o que impede, mundo, tom, final possível), devolver o que entendeu, propor cartões ligados a cada ponto que firmar, propor um nome quando a história tiver forma e não escrever o livro. Instrução comum: o mapa de ideias é da pessoa; use `propor_cartoes` para sugerir, nunca diga que alterou o mapa.
  - Comandos: `map_proposals_pending(book_id)`, `map_proposal_accept(id, refs: Vec<String>) -> Vec<MapCard>`, `map_proposal_discard(id)`.

- [ ] **Step 1: Testes que falham**
  - `map.rs`: `accept_places_cards_right_of_existing_in_grid` (existente em (100, 50) → 1º novo em (420, 50), 4º em (420, 230)); `accept_subset_skips_links_to_unchosen`; `accept_ignores_link_to_deleted_card` (Review Focus 2); `accept_twice_fails_without_duplicates` (Review Focus 3); `name_proposal_renames_and_clears_provisional`; `discard_sets_status`
  - `tools.rs`: `propor_cartoes_rejects_unknown_or_foreign_ids` (Review Focus 1: nada gravado em `map_proposals`); `propor_cartoes_works_in_conselheira`; `propor_nome_only_in_idea_conversation`; `ler_mapa_lists_cards_and_links`
  - `chat.rs`: `map_proposal_emits_event` (molde de `coautora_proposal_emits_proposal_event`)
  - `context.rs`: `idea_conversation_gets_idea_instructions` e `outline_is_called_sumario`
- [ ] **Step 2:** `cargo test --lib --manifest-path src-tauri/Cargo.toml` → os novos falham.
- [ ] **Step 3:** Implemente.
- [ ] **Step 4:** `cargo test --lib --manifest-path src-tauri/Cargo.toml` → passa.
- [ ] **Step 5:** Commit `feat(ia): ler o mapa e propor cartões e nome`.

### Task 4: Componente do mapa

**Files:**
- Modify: `package.json` / `package-lock.json` (`npm i @vue-flow/core@^1.48.2`)
- Create: `src/features/map/flow.ts`, `src/features/map/flow.test.ts`, `src/features/map/MapView.vue`, `src/features/map/CardNode.vue`
- Modify: `src/styles/themes.css` (tokens `--map-papel` … `--map-rosa`, claro e escuro)

**Interfaces:**
- Consumes: API do Task 2.
- Produces:
  - `flow.ts`: `toNodes(cards: MapCard[]): Node[]` (type `card`, `data: { card }`), `toEdges(links: MapLink[]): Edge[]` (`markerEnd` seta, `label`), `newCardsBounds(cards: MapCard[], ids: string[])` para enquadrar.
  - `MapView.vue`: props `bookId: string`; emits `bibleChanged`; expõe `reload(): Promise<void>` e `focusCards(ids: string[]): void`.
- Comportamento (do spec): duplo clique no fundo cria cartão ali e já edita; botão "+ Cartão" cria no centro; duplo clique no cartão edita (textarea; Esc ou clicar fora salva); Delete/Backspace apaga o que estiver selecionado, exceto durante a edição; arrastar a alça até outro cartão liga; duplo clique na seta edita o rótulo; Shift seleciona vários; botão "Ver tudo" (`fitView`); arrastar salva a posição ao soltar (`card_move` em `onNodeDragStop`); menu do cartão (⋯) com 5 cores, "Transformar em personagem / lugar / fato" (chama `mapCardToBible`, mostra o selo com o tipo, emite `bibleChanged`) e "Apagar"; mapa vazio mostra a dica exata das Global Constraints. Visual nos tokens do tema; importe o CSS base do Vue Flow e sobrescreva cores e fontes.

- [ ] **Step 1: Testes que falham** em `flow.test.ts`: `toNodes keeps position and card data`; `toEdges maps label and arrow marker`; `newCardsBounds covers only given ids`.
- [ ] **Step 2:** `npx vitest run src/features/map` → falha.
- [ ] **Step 3:** Implemente `flow.ts`, `MapView.vue` e `CardNode.vue`.
- [ ] **Step 4:** `npx vitest run` e `npx vue-tsc --noEmit -p tsconfig.json` passam.
- [ ] **Step 5:** Commit `feat(mapa): mapa de ideias com cartões e setas`.

### Task 5: Aba Texto | Mapa no editor

**Files:**
- Modify: `src/features/editor/EditorScreen.vue`, `src/features/editor/shortcuts.ts`, `src/features/editor/shortcuts.test.ts`, `src/features/editor/ShortcutsHelp.vue`

**Interfaces:**
- Consumes: `MapView` (Task 4), carregado com `defineAsyncComponent(() => import("../map/MapView.vue"))`.
- Produces: `EditorScreen` props novas `initialView?: "texto" | "mapa"` e `idea?: string` (usadas no Task 7); `ref view: "texto" | "mapa"`; ação de atalho `toggleMap` (⌘⇧M / Ctrl+Shift+M, grupo `Navegar`, "Alternar entre texto e mapa").
- Seletor "Texto | Mapa" na barra do editor. No Mapa, a folha some e o sumário e o assistente continuam; os botões só de texto (Imagem, Versões) ficam desabilitados. `bibleChanged` atualiza a ficha aberta no assistente.

- [ ] **Step 1: Teste que falha** em `shortcuts.test.ts`: `toggleMap is mod+shift+m and does not clash` (nenhum outro atalho com a mesma combinação).
- [ ] **Step 2:** `npx vitest run src/features/editor` → falha.
- [ ] **Step 3:** Implemente a aba, o atalho e a linha na Ajuda.
- [ ] **Step 4:** `npx vitest run` e `vue-tsc` passam.
- [ ] **Step 5:** Commit `feat(editor): aba Texto | Mapa`.

### Task 6: Propostas da IA na conversa

**Files:**
- Create: `src/features/assistant/MapProposalCard.vue`, `src/features/assistant/MapProposalCard.test.ts`
- Modify: `src/features/assistant/ChatView.vue`, `src/features/assistant/ChatView.test.ts`, `src/features/assistant/AssistantPanel.vue`, `src/features/editor/EditorScreen.vue`

**Interfaces:**
- Consumes: `mapProposalsPending`, `mapProposalAccept`, `mapProposalDiscard`, evento `map_proposal` (Task 3); `MapView.reload` e `focusCards` (Task 4).
- Produces:
  - `MapProposalCard.vue`: props `proposal: MapProposal`; emits `accepted(cards: MapCard[])`, `renamed(title: string)`, `discarded()`.
    - `cartoes`: título "A IA sugere N cartões para o mapa", lista com caixa marcada por cartão e "+ M ligações", botões **Pôr no mapa** (desabilitado sem nenhum marcado, e durante a chamada) e **Descartar**.
    - `nome`: "Usar “X” como nome do livro?" com **Usar** e **Agora não**.
    - Erro da chamada aparece no próprio bloco.
  - `ChatView`: item `{ kind: "map_proposal" }` nos dois modos, vindo do evento e das pendentes ao abrir; emits `mapChanged(cardIds: string[])` e `bookRenamed()`; expõe `startIdea(text: string): Promise<void>` (`aiConversationNewIdea`, carrega a conversa e envia o texto como primeira mensagem).
  - `AssistantPanel` repassa os eventos e expõe `startIdea`.
  - `EditorScreen`: em `mapChanged`, muda para o Mapa, chama `reload()` e `focusCards(ids)`; em `bookRenamed`, chama `reloadOutline()`.
- [ ] **Step 1: Testes que falham**
  - `MapProposalCard.test.ts`: `lists cards checked and accepts only checked refs` (desmarca um → `mapProposalAccept` recebe só os outros); `accept disabled with none checked`; `name proposal shows title and emits renamed`.
  - `ChatView.test.ts`: `map_proposal event shows block in conselheira`.
- [ ] **Step 2:** `npx vitest run src/features/assistant` → falha.
- [ ] **Step 3:** Implemente.
- [ ] **Step 4:** `npx vitest run` e `vue-tsc` passam.
- [ ] **Step 5:** Commit `feat(ia): aprovar cartões e nome sugeridos na conversa`.

### Task 7: Tela "Novo livro" e capa provisória

**Files:**
- Modify: `src/features/library/Library.vue`, `src/App.vue`
- Create: `src/features/library/Library.test.ts`

**Interfaces:**
- Consumes: `bookCreate` (nome pode ir vazio), `aiKeyStatus`, props `initialView` e `idea` do Task 5, `startIdea` do Task 6.
- Produces: `Library` emite `open(id: string, opts?: { idea?: string })`. `App.openBook(id, opts)` guarda `opts` e repassa `initialView="mapa"` e `idea` ao `EditorScreen`; o `EditorScreen`, quando recebe `idea`, abre o assistente e chama `startIdea(idea)` uma única vez.
- Diálogo conforme o esboço do spec:
  - Rótulo "Nome do livro", dica "Pode ser um nome provisório. Dá para trocar depois."
  - "Como você quer começar?": **Escrever direto** (padrão) ou **Tenho uma ideia e quero desenvolver com a IA**, que mostra um textarea com placeholder "Conte a ideia do jeito que vier…".
  - Sem chave de IA, a segunda opção fica desligada com "Para usar, configure a IA em Ajustes do assistente".
  - "Criar livro" só fica desligado quando a opção da ideia está escolhida e o textarea está vazio.
  - A capa mostra "nome provisório" quando `title_provisional`.
- [ ] **Step 1: Testes que falham** em `Library.test.ts` (com `api` mockado):
  - `creates with empty name`: `bookCreate` chamado com `""`.
  - `idea option disabled without ai key`.
  - `idea option requires text and emits open with idea`.
  - `provisional cover shows tag`.
- [ ] **Step 2:** `npx vitest run src/features/library` → falha.
- [ ] **Step 3:** Implemente.
- [ ] **Step 4:** `npx vitest run` e `vue-tsc` passam.
- [ ] **Step 5:** Commit `feat(biblioteca): nome provisório e começar desenvolvendo a ideia com a IA`.

### Task 8: Verificação de ponta a ponta

- [ ] **Step 1:** Rode as três suítes: `npx vitest run`, `cargo test --lib --manifest-path src-tauri/Cargo.toml` e `npx vue-tsc --noEmit -p tsconfig.json`. Todas devem passar.
- [ ] **Step 2:** Rode `npm run dev` (mock) e capture telas com playwright-core e o Chrome do sistema, nos temas claro e escuro: diálogo novo, mapa vazio com a dica, mapa com cartões e setas, bloco de proposta na conversa e capa com "nome provisório". Confira legibilidade e alinhamento e corrija o que estiver errado.
- [ ] **Step 3:** Rode `npx tauri build` e instale em /Applications. No app real, crie um livro pela opção da ideia (com a chave configurada), aceite cartões e um nome, feche, reabra e confira que tudo continua lá.
- [ ] **Step 4:** Commit das correções, se houver.
