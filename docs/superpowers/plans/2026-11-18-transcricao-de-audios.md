# Transcrição dos áudios — Plano

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Áudios que chegam do celular são transcritos pela Inworld, literalmente e em português, e o texto pode virar cena, entrar no fim da cena atual ou abrir uma conversa com o Assistente.

**Architecture:** Rust: colunas novas na tabela local `inbox`; módulo `ai::stt` com um `Transcriber` (Inworld via multipart, igual à chave Basic do chat) e uma fila `transcribe_pending` que tranca o banco só por item. Frontend: o painel Ideias chama a fila depois de cada sincronização e mostra texto/estado; as ações reaproveitam `insertAtEnd` do editor e abrem o Assistente com um rascunho.

**Tech Stack:** Rust (`reqwest` com feature `multipart`), Vue 3/TS, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-18-transcricao-de-audios-design.md`

## Global Constraints

- Endpoint: `POST https://api.inworld.ai/stt/v1/transcribe`, header `Authorization: Basic <chave>`, multipart com a parte `transcribeConfig` (JSON) primeiro e depois `file`.
- `transcribeConfig`: `{"modelId":"inworld/inworld-stt-1","audioEncoding":"AUTO_DETECT","language":"pt","prompts":[...]}`; `prompts` só com letras, dígitos e espaços, no máximo 100, sem repetidos.
- Resposta: `transcription.transcript` (texto) e `usage.transcribedAudioMs`.
- Limite síncrono: 16 MiB (`16 * 1024 * 1024` bytes). Acima disso: `erro` com "Áudio longo demais para transcrever por enquanto." sem chamar a API.
- Estados (`transcript_status`): `pendente` | `pronta` | `erro` | `sem_chave`. Só itens `kind = 'audio'` têm estado; texto fica `NULL`.
- Texto literal: nenhum pós-processamento além de `trim()`.
- Uso: `usage_add(c, mês, "inworld/inworld-stt-1", audio_ms, 0, None)`; a tela converte em minutos e **não** soma esse modelo nos tokens.
- A chave nunca sai do Rust.

## Review Focus

1. **Chave inválida (401) ou sem saldo:** o item vai para `erro` com mensagem clara; o áudio continua tocável; nada de laço de tentativas. → teste Task 3.
2. **Sincronizações seguidas** não disparam duas transcrições do mesmo áudio ao mesmo tempo. → teste Task 3 (trava) e Task 4.
3. **Áudio sem fala:** `pronta` com texto vazio; o cartão diz "Não encontrei fala neste áudio" e esconde as ações de texto. → teste Task 5.
4. **Chave configurada depois:** itens `sem_chave` voltam para `pendente`. → teste Task 1.
5. **"Inserir no fim" com outra cena aberta:** entra na cena **aberta** (a ideia não pertence a cena nenhuma), em um ⌘Z. → verificação Task 5.

---

### Task 1: Banco — colunas de transcrição e vocabulário

**Files:** Modify `src-tauri/src/db/mod.rs` (migração nova), `src-tauri/src/db/inbox.rs`, `src-tauri/src/db/ai.rs`

**Interfaces — Produces:**
```rust
// migração: ALTER TABLE inbox ADD COLUMN transcript TEXT; ... transcript_status TEXT; ... transcript_error TEXT;
//           UPDATE inbox SET transcript_status='pendente' WHERE kind='audio';
// InboxItem ganha: transcript: Option<String>, transcript_status: Option<String>, transcript_error: Option<String>
pub fn inbox_insert(...)                      // áudio novo nasce com transcript_status = 'pendente'
pub fn inbox_pending_audio(c) -> Result<Vec<String>, DbError>;            // ids 'pendente', mais antigos primeiro
pub fn inbox_set_transcript(c, id, status: &str, text: Option<&str>, error: Option<&str>) -> Result<(), DbError>;
pub fn inbox_mark_no_key(c) -> Result<usize, DbError>;                    // pendente -> sem_chave
pub fn inbox_wake_no_key(c) -> Result<usize, DbError>;                    // sem_chave -> pendente
pub fn inbox_retry_transcript(c, id) -> Result<(), DbError>;              // erro|sem_chave -> pendente
pub fn bible_vocabulary(c) -> Result<Vec<String>, DbError>;               // em db/ai.rs: nomes e apelidos de personagem/lugar de todos os livros, sanitizados, únicos, máx. 100
// inbox_to_scene: áudio com transcript 'pronta' e não vazio passa a virar cena (texto = transcript); senão NOT_TEXT
```

- [ ] **Step 1: Testes que falham:** áudio inserido nasce `pendente`, texto nasce `NULL`; `inbox_pending_audio` só traz `pendente`, em ordem; `inbox_set_transcript` grava os três campos; `mark_no_key`/`wake_no_key` trocam só entre esses estados; `retry` aceita `erro` e `sem_chave`, recusa `pronta` (`Invalid`); `inbox_list` traz os campos novos; `inbox_to_scene` com áudio `pronta` cria cena com o texto e com áudio `pendente` dá `NOT_TEXT`; `bible_vocabulary` tira pontuação ("Rafa!" → "Rafa"), não repete, ignora fatos e linha do tempo, corta em 100; migração roda num banco com áudios antigos e os deixa `pendente`.
- [ ] **Step 2:** `cargo test`. Esperado: FAIL.
- [ ] **Step 3:** Implementar.
- [ ] **Step 4:** `cargo test`. Esperado: PASS.
- [ ] **Step 5:** Commit `feat(transcricao): estado da transcrição na caixa de ideias`.

### Task 2: Cliente Inworld STT

**Files:** Create `src-tauri/src/ai/stt.rs`; Modify `src-tauri/src/ai/mod.rs`, `src-tauri/Cargo.toml` (`reqwest` + feature `multipart`)

**Interfaces — Produces:**
```rust
pub struct Transcript { pub text: String, pub audio_ms: i64 }
pub trait Transcriber: Send + Sync {
    fn transcribe(&self, audio: &[u8], mime: &str, vocabulary: &[String]) -> Result<Transcript, ProviderError>;
}
pub fn stt_config(vocabulary: &[String]) -> serde_json::Value;
pub fn parse_stt_response(body: &str) -> Result<Transcript, ProviderError>;
pub struct InworldStt { pub key: String }   // impl Transcriber; erros de status via ai::provider::status_error
pub const STT_MODEL: &str = "inworld/inworld-stt-1";
pub const STT_MAX_BYTES: usize = 16 * 1024 * 1024;
```

- [ ] **Step 1: Testes que falham:** `stt_config` tem `modelId`, `audioEncoding: AUTO_DETECT`, `language: pt` e os `prompts`; com vocabulário vazio não manda `prompts`; `parse_stt_response` lê texto e ms, aceita resposta sem `usage` (ms = 0) e faz `trim`; JSON quebrado → erro. Teste `#[ignore]` `live_inworld_stt` (lê `INWORLD_KEY`, transcreve `test-fixtures/fala-pt.m4a`, espera texto não vazio).
- [ ] **Step 2:** `cargo test ai::stt`. Esperado: FAIL.
- [ ] **Step 3:** Implementar; gerar `test-fixtures/fala-pt.m4a` com `say -v Luciana "A Marina encontrou a carta do Teodoro." -o /tmp/f.aiff && afconvert -f m4af -d aac /tmp/f.aiff test-fixtures/fala-pt.m4a`.
- [ ] **Step 4:** `cargo test ai::stt`. Esperado: PASS (o ignorado não roda).
- [ ] **Step 5:** Commit `feat(transcricao): cliente Inworld STT`.

### Task 3: Fila de transcrição

**Files:** Create `src-tauri/src/ai/transcribe.rs`; Modify `src-tauri/src/ai/mod.rs`

**Interfaces — Produces:**
```rust
/// Transcreve os áudios pendentes, um por vez. Tranca o banco só para ler o item e para gravar o resultado.
/// `t = None` (sem chave) marca os pendentes como `sem_chave`. Devolve quantos ficaram `pronta`.
pub fn transcribe_pending(db: &SharedDb, t: Option<&dyn Transcriber>, now: i64) -> Result<usize, String>;
```

- [ ] **Step 1: Testes que falham** (`FakeTranscriber` que registra chamadas):
  - dois pendentes → os dois `pronta` com o texto; uso do mês com `STT_MODEL` soma os ms
  - transcritor falhando com 401 → `erro` com a mensagem, sem nova tentativa na mesma chamada, áudio intacto (`inbox_body` igual)
  - áudio maior que `STT_MAX_BYTES` → `erro` "Áudio longo demais…", transcritor nem é chamado
  - `t = None` → pendentes viram `sem_chave`, nada chamado
  - item já `pronta` não é transcrito de novo
  - vocabulário da Bíblia chega ao transcritor
  - duas chamadas simultâneas (`static` `AtomicBool`): a segunda devolve `Ok(0)` sem chamar nada
- [ ] **Step 2:** `cargo test ai::transcribe`. Esperado: FAIL.
- [ ] **Step 3:** Implementar.
- [ ] **Step 4:** `cargo test`. Esperado: PASS.
- [ ] **Step 5:** Commit `feat(transcricao): fila em segundo plano`.

### Task 4: Comandos

**Files:** Modify `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src/lib/api.ts`, `src/lib/types.ts`, `src/dev/mockBackend.ts`

**Interfaces — Produces:**
- `inbox_transcribe_pending() -> usize` (async, `spawn_blocking`; lê a chave no Rust; sem chave passa `None`).
- `inbox_retry_transcript(id)`.
- `ai_key_save` também chama `inbox_wake_no_key`.
- TS: `inboxTranscribePending()`, `inboxRetryTranscript(id)`; `InboxItem` ganha `transcript`, `transcript_status`, `transcript_error`.

- [ ] **Step 1: Teste que falha:** `commands.rs` — depois de `ai_key_save`, itens `sem_chave` voltam a `pendente`.
- [ ] **Step 2:** `cargo test`. Esperado: FAIL.
- [ ] **Step 3:** Implementar; mock do navegador simula a transcrição ("Texto transcrito de exemplo…").
- [ ] **Step 4:** `cargo test` e `npx vue-tsc --noEmit`. Esperado: PASS.
- [ ] **Step 5:** Commit `feat(transcricao): comandos`.

### Task 5: Interface

**Files:** Modify `src/features/inbox/InboxPanel.vue`, `src/features/editor/EditorScreen.vue`, `src/features/assistant/AssistantPanel.vue`, `src/features/assistant/ChatView.vue`, `src/features/editor/SettingsPanel.vue`; Create `src/features/inbox/ideaText.ts`; Test `src/features/inbox/ideaText.test.ts`, `src/features/editor/SettingsPanel.test.ts`, `src/features/assistant/ChatView.test.ts`

**Interfaces — Produces:**
```ts
// ideaText.ts
export function ideaText(i: InboxItem): string | null;          // texto da ideia (escrita ou transcrição pronta não vazia)
export function transcriptLabel(i: InboxItem): string | null;   // "Transcrevendo…", "Não encontrei fala neste áudio", erro, "Configure a chave…"
export function aboutIdea(text: string): string;                // "Sobre esta ideia: «…»\n\n"
// InboxPanel emits: insertText(text: string), discuss(text: string)
// AssistantPanel/ChatView: prop `draft?: string` que preenche o campo (não envia)
```
- Card de áudio: player + texto (corte em 6 linhas com "ver tudo") ou o rótulo de estado; "Tentar de novo" em `erro`.
- Ações quando `ideaText` existe: **Virar cena aqui**, **Inserir no fim da cena**, **Conversar sobre isso**, **Arquivar**.
- `InboxPanel` chama `inboxTranscribePending()` depois de cada `remoteSync` e recarrega; `EditorScreen.backgroundSync` também.
- `insertText` → `typewriter.insertAtEnd(text)`; sem cena aberta mostra "Abra uma cena para inserir a ideia."
- `discuss` → abre o Assistente com `draft = aboutIdea(text)`.
- Ajustes: "X min de transcrição este mês" e os tokens **sem** o modelo `inworld/inworld-stt-1`.

- [ ] **Step 1: Testes que falham:** `ideaText` (texto, áudio pronto, áudio vazio → null, pendente → null); `transcriptLabel` para cada estado; `aboutIdea`; `SettingsPanel` mostra minutos (90.000 ms → "2 min") e não soma o STT nos tokens; `ChatView` com `draft` preenche o campo e não envia.
- [ ] **Step 2:** `npx vitest run`. Esperado: FAIL.
- [ ] **Step 3:** Implementar.
- [ ] **Step 4:** `npx vitest run` e `npx vue-tsc --noEmit`. Esperado: PASS.
- [ ] **Step 5: Verificação manual** no navegador com o mock: cartão de áudio com texto, "Inserir no fim" entra na cena aberta, "Conversar sobre isso" abre o Assistente com o rascunho.
- [ ] **Step 6:** Commit `feat(transcricao): texto no cartão e ações`.

### Task 6: Teste real e entrega

- [ ] **Step 1:** Teste real com a chave do cofre do usuário via exemplo `examples/stt_check.rs` (abre o cofre com `VAULT_PASSWORD`, lê a chave sem imprimi-la, transcreve `test-fixtures/fala-pt.m4a` e um áudio webm gerado). Esperado: texto em português com "Marina" e "Teodoro".
- [ ] **Step 2:** Todas as suítes: `npx vitest run`, `cargo test`, `npm --prefix server test`, `npm --prefix capture test`.
- [ ] **Step 3:** `npm run tauri build`, instalar em `/Applications` (com o app fechado).
- [ ] **Step 4:** Commit `chore(transcricao): teste real e entrega`.
