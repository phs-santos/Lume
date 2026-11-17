# Plano 3 — Assistente de IA e Bíblia viva — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Painel de IA à direita do editor, com conversa persistente por livro, Bíblia viva atualizada em segundo plano e propostas da Coautora, de forma econômica.

**Architecture:** Toda a IA mora no Rust (`src-tauri/src/ai/`), que lê a chave do banco cifrado, monta o contexto, chama a Inworld (OpenAI-compatível) e grava tudo no SQLCipher. O provedor fica atrás de um trait `ChatModel`, para os testes usarem um modelo falso. O front (Vue) só mostra o painel e recebe o streaming por `tauri::ipc::Channel`.

**Tech Stack:** Rust (rusqlite/SQLCipher, reqwest blocking, serde_json), Tauri 2, Vue 3 + TS, Vitest + happy-dom.

**Spec:** `docs/superpowers/specs/2026-11-17-plano-3-assistente-e-biblia-design.md` (complementa `2026-11-17-maquina-de-escrever-design.md` §7.3–7.4)

## Global Constraints
- Inworld: `POST https://api.inworld.ai/v1/chat/completions`, header `Authorization: Basic <chave>`, `stream: true` (SSE), `tools` no formato OpenAI. Lista de modelos: `GET https://api.inworld.ai/llm/v1alpha/models`.
- Modelos padrão: conversa `anthropic/claude-sonnet-5-5`; fundo `anthropic/claude-haiku-5-5`.
- Chaves de settings: `ai.inworld_key`, `ai.chat_model`, `ai.background_model`, `ai.enabled.<book_id>` (`"1"` quando o Assistente já foi aberto naquele livro).
- **A chave nunca sai do Rust.** Nenhum comando devolve a chave à webview; há só `ai_key_set(key)` e `ai_key_status() -> bool`.
- **Nunca segurar o `Mutex` do banco durante uma chamada de rede.** Ler o que precisa, soltar, chamar a IA, travar de novo para gravar (padrão de `remote_sync`).
- Conselheira: a lista de ferramentas enviada ao modelo **não contém** `propor_trecho`, e o executor recusa essa ferramenta se ela vier mesmo assim.
- Nada escreve em `scenes` a partir da IA. A inserção de propostas é feita pelo front, no editor aberto.
- Janela da conversa: últimas **12** mensagens na íntegra. Máximo de **6** chamadas de ferramenta por resposta.
- Reprocessar cena: ao sair dela ou após **2 min** sem digitar, e só se o hash do texto mudou.
- Textos da interface em português do Brasil; ícones com `title` e `aria-label`.

## Review Focus
- Chave inválida, sem crédito ou sem rede: o painel mostra o erro com "Tentar de novo", e a mensagem da pessoa **não se perde** (fica salva antes da chamada).
- Fechar o app no meio de uma resposta em streaming: a conversa reabre com a pergunta salva e sem resposta pela metade corrompida (a resposta parcial é descartada ou marcada como interrompida).
- A IA chamando a mesma ferramenta em laço: o limite de 6 encerra e ela responde com o que tem.
- JSON malformado do agente de fundo: a cena fica pendente, nada na Bíblia muda, e não há laço de novas tentativas imediatas.
- Livro de 50+ cenas na primeira passada: roda em fila, uma cena por vez, cancelável, e o editor continua utilizável (sem travar o banco).

---

### Task 1: Tabelas e armazenamento da IA

**Files:**
- Modify: `src-tauri/src/db/mod.rs` (nova migração no fim de `MIGRATIONS`; `pub mod ai;`)
- Create: `src-tauri/src/db/ai.rs`

**Interfaces:**
- Produces (em `crate::db::ai`, todos `-> Result<_, DbError>`):
  - Tabelas conforme spec §7.4. `bible_entries`: `aliases_json TEXT` (array JSON), `ai_notes_json TEXT` (array de `{fact, source_scene_id}`), `story_order INTEGER NULL`, `edited_by_user INTEGER`. `source_scene_id` com `ON DELETE SET NULL` (ou limpo em `scene_delete`).
  - `struct BibleEntry { id, book_id, kind, name, aliases: Vec<String>, body, ai_notes: Vec<AiNote>, story_order: Option<i64>, source_scene_id: Option<String>, edited_by_user: bool, updated_at }` (+ `AiNote { fact, source_scene_id }`), serde camel/snake igual aos outros tipos (snake_case, como `Scene`).
  - `bible_list(c, book_id) -> Vec<BibleEntry>` (linha do tempo por `story_order`, demais por nome)
  - `bible_find(c, book_id, kind, name_or_alias) -> Option<BibleEntry>`: compara sem maiúsculas e sem acentos, com o nome **e** os apelidos
  - `bible_upsert_ai(c, book_id, kind, name, aliases, fact, source_scene_id, story_order, now) -> BibleEntry`: acha por `bible_find`; se não existe, cria com `body = fact`. Se existe e `!edited_by_user`, reescreve `body` com `fact` e une os apelidos. Se `edited_by_user`, só acrescenta `{fact, source}` em `ai_notes` (sem duplicar fato idêntico).
  - `bible_update_user(c, id, name, aliases, body, story_order, now)` → marca `edited_by_user = 1`
  - `bible_delete(c, id)`
  - `digest_get(c, scene_id) -> Option<(hash, summary)>`, `digest_put(c, scene_id, hash, summary, now)`
  - `conversation_current(c, book_id, now) -> Conversation` (cria se não houver aberta), `conversation_new(c, book_id, now)` (fecha a atual), `conversation_list(c, book_id)`, `conversation_set_summary(c, id, summary)`, `conversation_open(c, id)` (reabre a escolhida e fecha a atual)
  - `message_add(c, conversation_id, role, content, tool_name: Option<&str>, now) -> Message`, `messages_all(c, conversation_id)`, `message_delete(c, id)`
  - `proposal_add(...) -> Proposal`, `proposal_set_status(c, id, status)`, `proposals_pending(c, book_id)`
  - `usage_add(c, month: &str, model, input_tokens, output_tokens, cost: Option<f64>)`, `usage_month(c, month) -> Vec<UsageRow>`

- [ ] **Step 1: Testes** em `db/ai.rs`:
  - `migration_adds_ai_tables_and_keeps_books`: abrir um banco v2 com um livro → `user_version` 3, livro intacto
  - `find_matches_alias_ignoring_case_and_accents`: ficha "Rafael" com apelido "Rafa" → achada por "rafa" e por "RAFAÉL"
  - `upsert_creates_then_updates_same_entry`: duas chamadas com "Rafa" e "Rafael" → 1 ficha só, com os apelidos unidos
  - `upsert_on_user_edited_only_appends_note`: após `bible_update_user`, um `bible_upsert_ai` mantém o `body` e acrescenta 1 nota; repetir o mesmo fato não duplica
  - `timeline_sorted_by_story_order`
  - `deleting_scene_nulls_source`
  - `conversation_new_closes_current_and_list_returns_both`
  - `digest_put_overwrites`
- [ ] **Step 2:** `cd src-tauri && cargo test db::ai` → FAIL (módulo não existe)
- [ ] **Step 3:** Implementar a migração e as funções acima.
- [ ] **Step 4:** `cargo test db::` → PASS (inclui os testes antigos de migração)
- [ ] **Step 5:** Commit `feat(ia): tabelas da Bíblia, conversas, propostas e uso`

### Task 2: Texto da cena e provedor Inworld

**Files:**
- Create: `src-tauri/src/ai/mod.rs`, `src-tauri/src/ai/text.rs`, `src-tauri/src/ai/provider.rs`
- Modify: `src-tauri/src/lib.rs` (`pub mod ai;`)

**Interfaces:**
- Produces:
  - `text::plain_text(content_json: &str) -> String`: parágrafos do ProseMirror separados por `\n\n`; JSON inválido vira `""`
  - `text::content_hash(content_json: &str) -> String`: hash estável do **texto plano** (formatação não conta), hex de SHA-256 ou de `std::hash` 64-bit
  - `provider::{Msg, ToolCall, ToolDef, Turn, Usage, ChatModel, ProviderError}`:
    - `Msg { role: "system"|"user"|"assistant"|"tool", content: String, tool_calls: Vec<ToolCall>, tool_call_id: Option<String> }`
    - `ToolCall { id, name, arguments_json }`, `ToolDef { name, description, parameters: serde_json::Value }`
    - `Turn { text: String, tool_calls: Vec<ToolCall>, usage: Usage }`, `Usage { input_tokens: i64, output_tokens: i64, cost: Option<f64> }`
    - `trait ChatModel: Send + Sync { fn complete(&self, model: &str, msgs: &[Msg], tools: &[ToolDef], on_delta: &mut dyn FnMut(&str)) -> Result<Turn, ProviderError>; }`
    - `ProviderError::{Auth, Quota, Network(String), Bad(String)}`, com `Display` em pt-BR curto (`"Chave inválida"`, `"Sem crédito ou limite atingido"`, `"Sem conexão"`, …)
  - `provider::Inworld { key: String }` implementa `ChatModel` com `stream: true` e `stream_options.include_usage: true`. Ele junta os deltas de `tool_calls` por `index`.
  - `provider::parse_sse(body: impl BufRead, on_delta) -> Result<Turn, ProviderError>`, pública para testes.
- Dependência nova: `sha2 = "0.10"` (se usar SHA-256). Desligar `blocking` não muda: o streaming lê `Response` como `BufRead` linha a linha.

- [ ] **Step 1: Testes**:
  - `plain_text_joins_paragraphs` (doc com 2 parágrafos e um itálico → `"A b\n\nC"`)
  - `hash_ignores_formatting` (mesmo texto com e sem itálico → mesmo hash; texto diferente → hash diferente)
  - `parse_sse_text_and_usage` (fixture com 3 chunks de `content` + chunk de usage + `[DONE]` → texto unido, usage preenchido, `on_delta` chamado 3×)
  - `parse_sse_tool_call_split_across_chunks` (nome no 1º chunk, argumentos em 2 pedaços → 1 `ToolCall` com JSON completo)
  - `http_status_maps_to_error` (401 → `Auth`, 402/429 → `Quota`)
- [ ] **Step 2:** `cargo test ai::` → FAIL
- [ ] **Step 3:** Implementar.
- [ ] **Step 4:** `cargo test ai::` → PASS
- [ ] **Step 5: Teste manual real** (ignorado no CI): `#[ignore] fn live_inworld_says_ok()`, lendo a chave de `INWORLD_KEY`. Rodar com `INWORLD_KEY=… cargo test live_inworld -- --ignored` → PASS.
- [ ] **Step 6:** Commit `feat(ia): texto da cena e cliente Inworld com streaming`

### Task 3: Ferramentas e montagem do contexto

**Files:**
- Create: `src-tauri/src/ai/tools.rs`, `src-tauri/src/ai/context.rs`

**Interfaces:**
- Consumes: Task 1 (`db::ai`), Task 2 (`Msg`, `ToolDef`, `plain_text`), `db::books::book_outline`, `db::scenes::scene_get`.
- Produces:
  - `tools::defs(mode: &str) -> Vec<ToolDef>`: `buscar_no_livro{texto}`, `ler_cena{id}`, `ler_ficha{id_ou_nome}`, `salvar_ficha{id?, tipo, nome, apelidos?, fato, cena_origem?, ordem?}`, `ler_conversas_anteriores{}`, e só se `mode == "coautora"`: `propor_trecho{texto}`
  - `tools::run(c, book_id, mode, current_scene_id, conversation_id, call: &ToolCall, now) -> ToolOutcome`, com `ToolOutcome { result: String, note: String, proposal: Option<Proposal> }`. `note` é a linha discreta da interface (`"consultei a cena X"`, `"atualizei a ficha Rafa"`). Erros de argumento viram `result` de erro para o modelo, nunca panic.
  - `buscar_no_livro`: até 8 trechos de ~300 caracteres em volta da ocorrência (sem diferenciar maiúsculas e acentos), com título de cena e capítulo
  - `context::build(c, book_id, mode, current_scene_id, conversation_id) -> Vec<Msg>`, nesta ordem: system fixo (por modo) → Bíblia compacta (`id · tipo · nome (apelidos): 1ª linha do body`) → mapa do livro (capítulo › cena: resumo do digest ou "(sem resumo)") → cena atual (texto completo) → `rolling_summary` da conversa (se houver) → últimas 12 mensagens
  - `context::needs_fold(msgs_total: usize) -> bool`: verdadeiro quando há mais de 24 mensagens fora do resumo
  - Texto exato das instruções fixas: quem implementa decide, desde que contenha estas regras:
    - responder em pt-BR
    - consultar antes de afirmar algo sobre o livro
    - nunca inventar fatos do livro
    - na Conselheira, deixar claro que não escreve no texto
    - registrar na Bíblia fatos novos e estáveis

- [ ] **Step 1: Testes**:
  - `conselheira_has_no_propose_tool` / `coautora_has_propose_tool`
  - `run_rejects_propose_in_conselheira` (call `propor_trecho` com mode conselheira → `result` de erro, nenhuma proposta criada)
  - `search_finds_accented_word_with_context`
  - `save_card_respects_user_edit` (via ferramenta → mesma regra do Task 1)
  - `context_order_and_window` (conversa com 20 mensagens → a saída termina com exatamente 12 mensagens na ordem; a 1ª msg é system; a cena atual aparece antes delas)
  - `context_without_digest_says_sem_resumo`
- [ ] **Step 2:** `cargo test ai::` → FAIL
- [ ] **Step 3:** Implementar.
- [ ] **Step 4:** `cargo test ai::` → PASS
- [ ] **Step 5:** Commit `feat(ia): ferramentas do agente e montagem de contexto`

### Task 4: Laço da conversa

**Files:**
- Create: `src-tauri/src/ai/chat.rs`

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces:
  - `enum ChatEvent { Delta{text}, Note{text}, Proposal{proposal}, Done{message_id}, Error{message} }` (serde com `tag = "type"`, `rename_all = "snake_case"`)
  - `chat::send(db: &SharedDb, model: &dyn ChatModel, cfg: &AiConfig, book_id, current_scene_id, text, emit: &mut dyn FnMut(ChatEvent), now: fn() -> i64) -> Result<(), String>`
    - Grava a mensagem do usuário **antes** de chamar a IA.
    - Repete `context::build` + `complete` + `tools::run` até vir uma resposta sem ferramentas ou até **6** chamadas de ferramenta. Ao atingir o limite, faz uma última chamada **sem tools**.
    - Grava a resposta final como `assistant`, soma `usage_add` em cada chamada e emite `Done`.
    - Em erro do provedor: emite `Error`, não grava resposta parcial, e a mensagem do usuário continua salva.
    - Não segura o lock durante `complete`.
  - `chat::retry(...)`: igual a `send`, mas reaproveita a última mensagem do usuário sem duplicá-la.
  - `chat::fold_if_needed(db, model, cfg, conversation_id)`: quando `needs_fold`, resume no modelo de fundo e grava `rolling_summary` (as mensagens continuam no banco, só saem da janela).
  - `AiConfig { chat_model, background_model }`, lido de settings com os padrões.

- [ ] **Step 1: Testes** com `FakeModel` (respostas roteirizadas):
  - `plain_answer_streams_and_saves`
  - `tool_then_answer_emits_note_and_saves_once`
  - `tool_loop_capped_at_6_then_final_without_tools` (fake que sempre pede ferramenta → 7 chamadas ao modelo, a última com `tools` vazio)
  - `provider_error_keeps_user_message_and_saves_no_answer`
  - `retry_does_not_duplicate_user_message`
  - `coautora_proposal_emits_proposal_event`
  - `usage_accumulates_per_call`
- [ ] **Step 2:** `cargo test ai::chat` → FAIL
- [ ] **Step 3:** Implementar.
- [ ] **Step 4:** `cargo test ai::` → PASS
- [ ] **Step 5:** Commit `feat(ia): laço da conversa com ferramentas, limite e memória`

### Task 5: Agente de fundo (resumos e Bíblia)

**Files:**
- Create: `src-tauri/src/ai/background.rs`

**Interfaces:**
- Consumes: Tasks 1–2, `context` (Bíblia compacta).
- Produces:
  - `background::digest_scene(db, model, cfg, scene_id, now) -> Result<bool, String>`: `false` quando o hash não mudou e nada foi chamado. Lê a cena e a Bíblia compacta e solta o lock. Chama o modelo de fundo pedindo **só JSON** `{"resumo": str, "fichas": [{"tipo","nome","apelidos":[],"fato","ordem"?}]}`. Valida e aplica cada ficha com `bible_upsert_ai` (origem = cena) e grava o digest. JSON inválido → `Err`, e nada é gravado.
  - `background::pending_scenes(c, book_id) -> Vec<String>`: cenas sem digest ou com hash diferente, na ordem do livro
  - `background::run_queue(db, model, cfg, book_id, cancel: &AtomicBool, progress: &mut dyn FnMut(done, total))`: processa uma por vez e para no cancelamento. Uma falha conta como feita, a cena fica pendente para a próxima vez, e a fila segue.

- [ ] **Step 1: Testes** com `FakeModel`:
  - `unchanged_scene_skips_model` (2ª chamada → `Ok(false)` e o fake recebeu 1 chamada só)
  - `applies_cards_and_summary`
  - `invalid_json_changes_nothing`
  - `queue_respects_cancel_and_reports_progress`
  - `pending_lists_new_and_changed_only`
- [ ] **Step 2:** `cargo test ai::background` → FAIL
- [ ] **Step 3:** Implementar.
- [ ] **Step 4:** `cargo test ai::` → PASS
- [ ] **Step 5:** Commit `feat(ia): agente de fundo para resumos e Bíblia viva`

### Task 6: Comandos Tauri e API do front

**Files:**
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` (registrar), `src/lib/api.ts`, `src/lib/types.ts`, `src/dev/mockBackend.ts` (respostas falsas para revisar a interface no navegador)

**Interfaces:**
- Produces (comandos; nomes TS em camelCase no `api.ts`):
  - `ai_key_set(key)`, `ai_key_status() -> bool`, `ai_models_get() -> {chat, background}`, `ai_models_set(chat, background)`, `ai_usage(month) -> Vec<UsageRow>`
  - `ai_enable(book_id)` → grava `ai.enabled.<id>`
  - `ai_send(book_id, scene_id: Option, text, on_event: Channel<ChatEvent>)` (async; `spawn_blocking`). Antes de responder, chama `fold_if_needed`.
  - `ai_retry(book_id, scene_id: Option, on_event: Channel<ChatEvent>)`
  - `ai_conversation(book_id) -> {conversation, messages}`, `ai_conversation_new(book_id)`, `ai_conversations(book_id)`, `ai_conversation_open(id)`
  - `bible_list(book_id)`, `bible_update(id, name, aliases, body, story_order)`, `bible_delete(id)`
  - `proposal_set_status(id, status)`, `proposals_pending(book_id)`
  - `ai_scene_touched(scene_id)`: dispara `digest_scene` em segundo plano **só se** o livro está habilitado e há chave; ignora se já houver um digest rodando para a cena
  - `ai_first_pass(book_id, on_progress: Channel<{done,total}>)` / `ai_first_pass_cancel()`: um `static AtomicBool` de cancelamento e um de "rodando", como o `SYNCING` existente
  - Sem chave: `ai_send` responde `Error{"Configure a chave da Inworld em Ajustes"}`

- [ ] **Step 1:** Teste em `commands.rs`: `ai_key_status_never_returns_key` (depois de `ai_key_set`, nenhum comando de leitura devolve a string da chave; `setting_get("ai.inworld_key")` pelo comando genérico **também** é bloqueado → `Err("FORBIDDEN")`)
- [ ] **Step 2:** `cargo test commands` → FAIL
- [ ] **Step 3:** Implementar os comandos, registrar em `lib.rs`, tipar em `api.ts`/`types.ts`, mockar em `mockBackend.ts`.
- [ ] **Step 4:** `cargo test` e `npx vue-tsc --noEmit` → PASS
- [ ] **Step 5:** Commit `feat(ia): comandos do assistente e da Bíblia`

### Task 7: Ajustes de IA

**Files:**
- Modify: `src/features/editor/SettingsPanel.vue`
- Test: `src/features/editor/SettingsPanel.test.ts`

- [ ] **Step 1: Testes** (happy-dom, `vi.mock("../../lib/api")`):
  - `shows_key_saved_state_without_value` (com `aiKeyStatus` → true, mostra "Chave salva" e o campo vazio)
  - `saving_key_calls_aiKeySet_and_clears_field`
  - `shows_month_usage_tokens`
- [ ] **Step 2:** `npx vitest run src/features/editor/SettingsPanel.test.ts` → FAIL
- [ ] **Step 3:** Implementar uma seção "Assistente de IA" no painel:
  - campo de senha para a chave, com "Salvar" e "Trocar"
  - dois `<select>` de modelo, com os padrões da Global Constraints e a opção "outro…" com campo de texto
  - linha "Este mês: N mil tokens · ~US$ X" (sem custo, mostra só tokens)
- [ ] **Step 4:** Rodar de novo → PASS
- [ ] **Step 5:** Commit `feat(ia): ajustes de chave, modelos e gasto`

### Task 8: Painel do Assistente — Conversa

**Files:**
- Create: `src/features/assistant/AssistantPanel.vue`, `src/features/assistant/ChatView.vue`
- Modify: `src/features/editor/EditorScreen.vue` (coluna à direita, ícone na barra, atalho), `src/features/editor/shortcuts.ts` (`action: "assistant"`, `key: "j"`, `mod: true`, `description: "Assistente de IA"`, `group: "Painéis"`; o teste de atalhos existente precisa continuar sem conflitos)
- Test: `src/features/assistant/ChatView.test.ts`

**Interfaces:**
- Consumes: Task 6.
- `AssistantPanel` props: `bookId`, `sceneId: string | null`, `mode`. Ele tem as abas Conversa/Bíblia e chama `aiEnable(bookId)` ao abrir.
- `EditorScreen`: a coluna do painel tem largura fixa (~22rem), a folha encolhe e o estado aberto/fechado fica em `settings` (`assistant_open`). Ao **sair de uma cena** e após **2 min sem digitar** (reaproveitar o `onChange` existente), chama `aiSceneTouched(sceneId)`.

- [ ] **Step 1: Testes**:
  - `renders_history_on_open`
  - `streams_deltas_into_last_bubble` (mock de `aiSend` que chama o channel com 2 deltas + done)
  - `shows_note_lines`
  - `error_shows_retry_and_keeps_user_message`
  - `no_key_shows_key_prompt`
  - `new_conversation_and_reopen_old`
- [ ] **Step 2:** `npx vitest run src/features/assistant` → FAIL
- [ ] **Step 3:** Implementar:
  - Enter envia e Shift+Enter quebra linha; o botão de enviar fica desativado durante o streaming.
  - A rolagem acompanha o fim da conversa.
  - Na primeira abertura num livro com cenas pendentes, mostra "Lendo o livro… x/y" com cancelar (`aiFirstPass`).
- [ ] **Step 4:** `npm test` → PASS
- [ ] **Step 5: Verificar no navegador** (`npm run dev`, mock): o painel abre pelo ícone e por ⌘J, a folha encolhe sem cortar e o painel não cobre a barra.
- [ ] **Step 6:** Commit `feat(ia): painel do assistente com conversa`

### Task 9: Aba Bíblia

**Files:**
- Create: `src/features/assistant/BibleView.vue`
- Test: `src/features/assistant/BibleView.test.ts`

**Interfaces:**
- Emits `openScene(sceneId)` (a origem da ficha); `EditorScreen` liga ao `openScene` existente.

- [ ] **Step 1: Testes**:
  - `groups_by_kind_and_filters_by_search` (busca acha também por apelido)
  - `edit_saves_with_bibleUpdate`
  - `shows_ai_notes_separately_on_user_edited`
  - `timeline_reorder_calls_bibleUpdate_with_new_order`
  - `source_link_emits_openScene`
- [ ] **Step 2:** `npx vitest run src/features/assistant/BibleView.test.ts` → FAIL
- [ ] **Step 3:** Implementar:
  - grupos Personagens / Lugares / Fatos / Linha do tempo
  - edição inline (nome, apelidos separados por vírgula, texto)
  - notas da IA com o botão "incorporar", que junta a nota ao texto e salva
  - apagar com confirmação
  - reordenar a linha do tempo com `vue-draggable-plus`, que já é dependência
- [ ] **Step 4:** Rodar de novo → PASS
- [ ] **Step 5:** Commit `feat(ia): aba Bíblia com edição e linha do tempo`

### Task 10: Propostas da Coautora

**Files:**
- Modify: `src/features/editor/Typewriter.vue` (expor `insertAtEnd(text: string)` via `defineExpose`), `src/features/editor/EditorScreen.vue`, `src/features/assistant/ChatView.vue`
- Test: `src/features/assistant/ChatView.test.ts` (novos casos), `src/features/editor/Typewriter.test.ts`

**Interfaces:**
- `Typewriter.insertAtEnd(text)`: cada bloco separado por linha em branco vira um parágrafo, no fim do documento, numa única transação (um ⌘Z desfaz tudo). Dispara o `change` normal, e o autosave grava.
- `ChatView` emite `insertProposal(proposal)` e `proposalAsScene(proposal)`. `EditorScreen` cuida dos dois:
  - **Inserir no fim da cena:** abre a cena da proposta se não for a atual, chama `insertAtEnd` e depois `proposalSetStatus(id, "inserida")`.
  - **Virar cena nova:** `sceneCreate` logo depois da cena da proposta, no mesmo capítulo, salva o conteúdo com `sceneSave`, abre a cena e marca `virou_cena`.
  - **Descartar:** marca `descartada`.

- [ ] **Step 1: Testes**:
  - `typewriter_insertAtEnd_adds_paragraphs_in_one_undo_step`
  - `proposal_card_buttons_emit` (3 botões)
  - `conselheira_never_shows_proposal_card` (evento de proposta ignorado no modo conselheira; defesa extra)
- [ ] **Step 2:** `npx vitest run` nesses arquivos → FAIL
- [ ] **Step 3:** Implementar.
- [ ] **Step 4:** `npm test` → PASS
- [ ] **Step 5:** Commit `feat(ia): propostas da coautora pelo editor`

### Task 11: Fechamento

- [ ] `cd src-tauri && cargo test` e `npm test` → tudo PASS.
- [ ] Teste real: rodar `INWORLD_KEY=… cargo test live_inworld -- --ignored` → PASS.
- [ ] `npx tauri build` e instalar em `/Applications`.
- [ ] Teste manual no app com a chave:
  - abrir "Até ter certeza"
  - abrir o Assistente e ver a primeira passada avançar
  - perguntar algo sobre um personagem e ver "consultei…"
  - fechar e reabrir o app: a conversa continua
  - conferir o gasto em Ajustes
