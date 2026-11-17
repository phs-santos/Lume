# Plano 1 — Editor Desktop (Máquina de Escrever)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Um app desktop onde a pessoa destrava seu cofre local com senha, cria livros, escreve cenas num editor de máquina de escrever, reordena cenas arrastando e exporta o livro para `.docx` e `.md`.

**Architecture:** Tauri 2: o lado Rust guarda tudo num SQLite cifrado (SQLCipher), cuja chave é uma chave-mestra aleatória embrulhada duas vezes (pela senha e pela frase de recuperação) num `vault.json`. O lado Vue 3/TypeScript fala com o Rust só por comandos `invoke` tipados; o editor é TipTap; a exportação roda no frontend.

**Tech Stack:** Tauri 2, Rust (rusqlite `bundled-sqlcipher-vendored-openssl`, argon2 0.5, chacha20poly1305 0.10, bip39 2, rand 0.8, zeroize, serde, uuid), Vue 3 (Composition API, `<script setup>`) + TypeScript + Vite, TipTap 2 (`@tiptap/vue-3`), vue-draggable-plus, docx (npm), tauri-plugin-dialog, tauri-plugin-fs, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-17-maquina-de-escrever-design.md` (seções 2.1, 4, 6.2, 6.3, 7.1)

**Fora deste plano:** contas no servidor, PWA, caixa de entrada, IA, backup na VPS (planos 2–4). O campo `ai_mode` do livro já existe aqui, mas nada o usa ainda.

## Global Constraints

- Plataformas: macOS e Windows.
- Toda a interface em português do Brasil.
- Nenhum dado do livro em texto puro no disco: só `vault.json` (sem conteúdo do livro) e o banco SQLCipher.
- KDF: Argon2id, m=65536 KiB, t=3, p=1, salt de 16 bytes, saída de 32 bytes.
- Embrulho de chave: XChaCha20-Poly1305, nonce de 24 bytes aleatório por embrulho.
- Frase de recuperação: BIP39, 24 palavras, lista em inglês (a lista PT do BIP39 é pouco suportada).
- `ai_mode` ∈ {`"conselheira"`, `"coautora"`}.
- Som de tecla: desligado por padrão.
- Dados ficam em `app_data_dir()` do Tauri: `vault.json` e `livro.db`.

## Review Focus

1. **Senha errada no desbloqueio:** mostra "Senha incorreta", não trava, não cria nem corrompe nada. → teste em Task 2.
2. **Frase de recuperação digitada com maiúsculas, espaços duplos ou quebra de linha:** é aceita depois de normalizada. → teste em Task 2.
3. **Fechar o app no meio da digitação:** o último trecho é salvo (o autosave é descarregado ao fechar a janela). → teste em Task 6.
4. **Mover cena para um capítulo vazio, para o fim, ou entre capítulos:** as posições continuam contíguas (0..n-1) nos dois capítulos. → teste em Task 3.
5. **Exportar texto com travessão de diálogo, acentos, `*`, `#` e `_` no meio da prosa:** sai idêntico no `.md` (com escape) e no `.docx`. → teste em Task 7.

Também decidido aqui (o design não diz nada sobre isso): **apagar capítulo só é permitido se ele estiver vazio.** A interface pede para mover as cenas antes, porque evitar perda de texto vale mais que a conveniência.

---

## Estrutura de arquivos

```
src-tauri/
  Cargo.toml
  src/main.rs              # registra comandos e estado
  src/state.rs             # AppState { db: Mutex<Option<Connection>> }
  src/vault.rs             # chave-mestra, embrulho, recuperação (puro, sem Tauri)
  src/db/mod.rs            # abrir SQLCipher + migrações
  src/db/books.rs          # livros e capítulos
  src/db/scenes.rs         # cenas, posições, versões
  src/db/settings.rs       # chave/valor de preferências
  src/commands.rs          # #[tauri::command] finos sobre vault/db
src/
  lib/types.ts             # tipos espelhando os structs Rust
  lib/api.ts               # wrappers tipados de invoke
  lib/wordcount.ts
  lib/outline.ts           # cálculo de destino no arrastar
  lib/export/markdown.ts
  lib/export/docx.ts
  features/vault/{CreateVault,Unlock,Recover}.vue
  features/library/Library.vue
  features/editor/{EditorScreen,Typewriter,Outline,SettingsPanel}.vue
  features/editor/useAutosave.ts   (composable)
  styles/themes.css        # temas "papel" e "escuro"
```

---

### Task 1: Esqueleto do app e cofre (`vault.rs`)

**Files:**
- Create: projeto via `npm create tauri-app@latest` (Vue + TS + Vite), `src-tauri/src/vault.rs`
- Test: testes `#[cfg(test)]` dentro de `vault.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct VaultFile { version: u32, nickname: String, pw_salt: [u8;16], rec_salt: [u8;16],
                         by_password: Wrapped, by_recovery: Wrapped }   // serde, bytes em base64
  pub struct Wrapped { nonce: [u8;24], ct: Vec<u8> }
  pub struct MasterKey(pub [u8;32]);   // Zeroize on drop
  pub fn create(nickname: &str, password: &str) -> Result<(VaultFile, MasterKey, String /*frase*/), VaultError>;
  pub fn unlock(v: &VaultFile, password: &str) -> Result<MasterKey, VaultError>;
  pub fn recover(v: &VaultFile, phrase: &str, new_password: &str) -> Result<(VaultFile, MasterKey), VaultError>;
  pub fn normalize_phrase(input: &str) -> String;
  pub enum VaultError { WrongPassword, InvalidPhrase, Io(String), Crypto }
  ```

- [ ] **Step 1:** `git init`, criar o app Tauri 2 (template vue-ts), adicionar as crates da Tech Stack no `Cargo.toml`. Rodar `npm run tauri dev` e ver a janela padrão abrir.
- [ ] **Step 2: Escrever os testes que falham**
  ```rust
  #[test] fn create_then_unlock_returns_same_key() {
      let (v, k, _) = create("ana", "senha-boa").unwrap();
      assert_eq!(unlock(&v, "senha-boa").unwrap().0, k.0);
  }
  #[test] fn wrong_password_is_wrongpassword() {
      let (v, _, _) = create("ana", "senha-boa").unwrap();
      assert!(matches!(unlock(&v, "errada"), Err(VaultError::WrongPassword)));
  }
  #[test] fn phrase_has_24_words() { let (_, _, p) = create("ana","x").unwrap(); assert_eq!(p.split(' ').count(), 24); }
  #[test] fn recover_sets_new_password_and_keeps_key() {
      let (v, k, p) = create("ana", "velha").unwrap();
      let (v2, k2) = recover(&v, &p, "nova").unwrap();
      assert_eq!(k.0, k2.0);
      assert_eq!(unlock(&v2, "nova").unwrap().0, k.0);
      assert!(unlock(&v2, "velha").is_err());
  }
  #[test] fn messy_phrase_is_accepted() {
      let (v, k, p) = create("ana", "x").unwrap();
      let messy = format!("  {}\n", p.to_uppercase().replace(' ', "  "));
      assert_eq!(recover(&v, &messy, "y").unwrap().1 .0, k.0);
  }
  #[test] fn wrong_phrase_is_invalidphrase() { /* frase BIP39 válida de outro cofre → InvalidPhrase */ }
  ```
- [ ] **Step 3:** Rodar `cargo test --manifest-path src-tauri/Cargo.toml vault`. Esperado: FAIL (funções não existem).
- [ ] **Step 4: Implementar `vault.rs`.** A chave-mestra são 32 bytes aleatórios. A chave de embrulho da senha é `Argon2id(password, pw_salt)`. A da recuperação é `Argon2id(entropia_bip39, rec_salt)`. Uma falha de AEAD com a senha vira `WrongPassword`; com a frase, vira `InvalidPhrase`. `normalize_phrase` faz trim, converte para minúsculas e colapsa espaços em branco. Também adicionar `load(path) -> Result<Option<VaultFile>>` e `save(path, &VaultFile)` com escrita atômica (arquivo temporário + rename).
- [ ] **Step 5:** Rodar os testes. Esperado: todos PASS.
- [ ] **Step 6:** Commit `feat: cofre local com chave-mestra e frase de recuperação`.

### Task 2: Banco cifrado, estado e comandos do cofre

**Files:**
- Create: `src-tauri/src/db/mod.rs`, `src-tauri/src/state.rs`, `src-tauri/src/commands.rs`, `src/lib/types.ts`, `src/lib/api.ts`
- Modify: `src-tauri/src/main.rs`
- Test: `#[cfg(test)]` em `db/mod.rs`

**Interfaces:**
- Consumes: `vault::{create, unlock, recover, load, save, MasterKey}`
- Produces:
  ```rust
  pub fn open(path: &Path, key: &MasterKey) -> Result<Connection, DbError>; // PRAGMA key = "x'<hex>'", depois migra
  // comandos Tauri (erros serializados como string com código: "WRONG_PASSWORD" | "INVALID_PHRASE" | "NO_VAULT" | "LOCKED" | mensagem)
  vault_status() -> "none" | "locked" | "unlocked"
  vault_create(nickname: String, password: String) -> String /*frase*/
  vault_unlock(password: String) -> ()
  vault_recover(phrase: String, new_password: String) -> ()
  vault_lock() -> ()
  ```
  ```ts
  // src/lib/api.ts — um wrapper por comando, mesmos nomes em camelCase:
  vaultStatus(), vaultCreate(nickname, password), vaultUnlock(password), vaultRecover(phrase, newPassword), vaultLock()
  ```
- Esquema (migração v1):
  ```sql
  books(id TEXT PK, title TEXT NOT NULL, ai_mode TEXT NOT NULL CHECK(ai_mode IN('conselheira','coautora')), created_at INTEGER NOT NULL)
  chapters(id TEXT PK, book_id TEXT NOT NULL REFERENCES books(id), title TEXT NOT NULL, position INTEGER NOT NULL)
  scenes(id TEXT PK, chapter_id TEXT NOT NULL REFERENCES chapters(id), title TEXT NOT NULL, content_json TEXT NOT NULL, word_count INTEGER NOT NULL DEFAULT 0, position INTEGER NOT NULL, updated_at INTEGER NOT NULL)
  scene_versions(id TEXT PK, scene_id TEXT NOT NULL REFERENCES scenes(id), content_json TEXT NOT NULL, word_count INTEGER NOT NULL, created_at INTEGER NOT NULL)
  settings(key TEXT PK, value TEXT NOT NULL)
  ```

- [ ] **Step 1: Testes que falham**
  - `open_with_right_key_reads_back`: abre, insere um livro, fecha, reabre com a mesma chave e lê.
  - `open_with_wrong_key_fails`: reabrir com outra `MasterKey` retorna erro, sem pânico.
  - `file_is_not_plaintext`: os bytes de `livro.db` não contêm o título `"Segredo da Marina"` inserido.
- [ ] **Step 2:** `cargo test db`. Esperado: FAIL.
- [ ] **Step 3:** Implementar `db::open` + migração v1 (`PRAGMA user_version`) + `AppState` + os comandos do cofre. `vault_unlock` com senha errada retorna `WRONG_PASSWORD` e não toca em nenhum arquivo.
- [ ] **Step 4:** `cargo test`. Esperado: PASS.
- [ ] **Step 5:** Commit `feat: banco SQLCipher e comandos do cofre`.

### Task 3: Livros, capítulos, cenas e posições

**Files:**
- Create: `src-tauri/src/db/books.rs`, `src-tauri/src/db/scenes.rs`, `src-tauri/src/db/settings.rs`
- Modify: `src-tauri/src/commands.rs`, `src/lib/types.ts`, `src/lib/api.ts`

**Interfaces:**
- Produces (Rust → comandos homônimos → wrappers TS em camelCase):
  ```rust
  struct Book { id, title, ai_mode, created_at }
  struct Chapter { id, book_id, title, position }
  struct SceneSummary { id, chapter_id, title, word_count, position }
  struct Scene { id, chapter_id, title, content_json, word_count, position, updated_at }
  struct Outline { book: Book, chapters: Vec<(Chapter, Vec<SceneSummary>)> }
  struct Version { id, created_at, word_count }

  book_list() -> Vec<Book>
  book_create(title, ai_mode) -> Book                 // já cria "Capítulo 1" com uma cena vazia "Cena 1"
  book_rename(id, title); book_set_mode(id, ai_mode)
  book_outline(book_id) -> Outline
  chapter_create(book_id, title, index: Option<i64>) -> Chapter   // None = no fim
  chapter_rename(id, title)
  chapter_move(id, to_index)
  chapter_delete(id)                                  // erro "CHAPTER_NOT_EMPTY" se tiver cenas
  scene_create(chapter_id, title, index: Option<i64>) -> Scene
  scene_get(id) -> Scene
  scene_rename(id, title)
  scene_save(id, content_json, word_count)            // grava versão se a última tiver > 10 min ou não existir
  scene_move(id, to_chapter_id, to_index)
  scene_delete(id)
  scene_versions(scene_id) -> Vec<Version>
  scene_restore(version_id) -> Scene                  // salva o estado atual como versão antes de restaurar
  setting_get(key) -> Option<String>; setting_set(key, value)
  ```

- [ ] **Step 1: Testes que falham** (`#[cfg(test)]` em `scenes.rs`, banco em arquivo temporário)
  - `new_book_has_one_chapter_one_scene`
  - `scene_create_with_index_inserts_in_middle`: cenas A,B,C; criar X no índice 1 → ordem A,X,B,C, posições 0..3.
  - `move_within_chapter`: A,B,C; mover C para 0 → C,A,B.
  - `move_to_other_chapter_keeps_both_contiguous`: cap1 A,B,C e cap2 D; mover B para cap2 índice 1 → cap1 A,C (0,1); cap2 D,B (0,1).
  - `move_to_empty_chapter`: cap2 vazio; mover A para índice 0 → cap2 [A] posição 0.
  - `move_index_past_end_clamps_to_end`
  - `delete_nonempty_chapter_fails`: retorna `CHAPTER_NOT_EMPTY` e nada muda.
  - `save_creates_version_only_after_10min`: dois saves seguidos → 1 versão; avançar o relógio injetado 11 min → 2 versões.
  - `restore_saves_current_first`
- [ ] **Step 2:** `cargo test scenes`. Esperado: FAIL.
- [ ] **Step 3:** Implementar. Toda mudança de posição roda numa transação que reescreve as posições do(s) capítulo(s) afetado(s) para 0..n-1. O relógio é injetável (`fn now() -> i64` passado como parâmetro nas funções de `db`) para o teste das versões.
- [ ] **Step 4:** `cargo test`. Esperado: PASS.
- [ ] **Step 5:** Commit `feat: livros, capítulos e cenas com reordenação e versões`.

### Task 4: Telas do cofre e biblioteca

**Files:**
- Create: `src/features/vault/{CreateVault,Unlock,Recover}.vue`, `src/features/library/Library.vue`, `src/App.vue` (roteamento por estado), `src/styles/themes.css`

**Interfaces:**
- Consumes: wrappers de `api.ts` das Tasks 2 e 3.

- [ ] **Step 1:** `App` consulta `vaultStatus()`: `"none"` → CreateVault; `"locked"` → Unlock; `"unlocked"` → Library.
- [ ] **Step 2:** **CreateVault**: nickname, senha e confirmação (mínimo de 8 caracteres). Depois de criar, mostra as 24 palavras numeradas com o texto *"Guarde estas palavras em papel. Sem elas e sem sua senha, ninguém — nem o administrador — consegue recuperar seus livros."* e só avança quando ela marca "Anotei as palavras".
- [ ] **Step 3:** **Unlock**: campo de senha. O erro `WRONG_PASSWORD` mostra "Senha incorreta". Tem um link "Esqueci minha senha" que leva a Recover.
- [ ] **Step 4:** **Recover**: textarea para a frase e nova senha. O erro `INVALID_PHRASE` mostra "Essas palavras não abrem este cofre".
- [ ] **Step 5:** **Library**: lista de livros, botão "Novo livro" (título + escolha do modo de IA: "Conselheira — a IA só sugere, nunca escreve no seu texto" / "Coautora — a IA escreve rascunhos que você aprova") e botão "Trancar".
- [ ] **Step 6: Verificação manual:** `npm run tauri dev` → criar cofre → anotar a frase → trancar → senha errada mostra erro → senha certa abre → recuperar com a frase em MAIÚSCULAS funciona → criar livro.
- [ ] **Step 7:** Commit `feat: telas de cofre e biblioteca`.

### Task 5: Sumário com arrastar e soltar

**Files:**
- Create: `src/lib/outline.ts`, `src/features/editor/Outline.vue`
- Test: `src/lib/outline.test.ts`

**Interfaces:**
- Produces:
  ```ts
  type DropTarget = { toChapterId: string; toIndex: number };
  // fromChapterId/toChapterId vêm do `data-chapter-id` das listas; newIndex vem do evento onEnd do SortableJS
  function resolveDrop(outline: Outline, sceneId: string, toChapterId: string, newIndex: number): DropTarget | null;
  ```
  `Outline.vue` props: `{ outline: Outline; currentSceneId: string }`; emits: `select(id: string)`, `changed()`.

- [ ] **Step 1: Testes que falham**
  - Mesmo capítulo, novo índice diferente → `{ toChapterId: mesmo, toIndex: newIndex }`.
  - Outro capítulo → `{ toChapterId: outro, toIndex: newIndex }`.
  - Capítulo vazio, índice 0 → `{ toChapterId: vazio, toIndex: 0 }`.
  - Mesmo capítulo e mesmo índice da posição atual → `null` (não chama o servidor).
  - `newIndex` maior que o tamanho da lista de destino → limitado ao fim.
- [ ] **Step 2:** `npx vitest run outline`. Esperado: FAIL.
- [ ] **Step 3:** Implementar `resolveDrop` e `Outline.vue` com `vue-draggable-plus`: uma lista arrastável por capítulo, todas no mesmo `group` para permitir mover entre capítulos. No `onEnd`, chamar `resolveDrop`; se o resultado não for `null`, chamar `sceneMove` e emitir `changed`. Se `sceneMove` falhar, recarregar o sumário para desfazer a mudança visual. Cada capítulo tem um menu com renomear, apagar (mostra "Mova as cenas antes de apagar o capítulo" se receber `CHAPTER_NOT_EMPTY`) e "+ cena". Entre duas cenas, um botão "+" discreto (no hover) cria uma cena naquele índice. Mostra a contagem de palavras por cena, por capítulo e o total do livro.
- [ ] **Step 4:** `npx vitest run`. Esperado: PASS.
- [ ] **Step 5:** Commit `feat: sumário com arrastar cenas entre capítulos`.

### Task 6: Editor máquina de escrever + autosave

**Files:**
- Create: `src/features/editor/{EditorScreen,Typewriter,SettingsPanel}.vue`, `src/features/editor/useAutosave.ts   (composable)`, `src/lib/wordcount.ts`
- Test: `src/lib/wordcount.test.ts`, `src/features/editor/useAutosave.test.ts`

**Interfaces:**
- Consumes: `sceneGet`, `sceneSave`, `sceneVersions`, `sceneRestore`, `settingGet/Set`.
- Produces:
  ```ts
  function countWords(text: string): number;
  function useAutosave(save: (json: string, words: number) => Promise<void>, delayMs = 800): {
    schedule(json: string, words: number): void; flush(): Promise<void>;
  };
  ```
  Chaves de configuração: `font` ∈ {`serif`,`mono`}, `theme` ∈ {`papel`,`escuro`}, `focus` ∈ {`on`,`off`}, `keysound` ∈ {`on`,`off`} (padrão `off`).

- [ ] **Step 1: Testes que falham**
  - `countWords("— Não sei, disse ela.")` = 4 (o travessão não conta como palavra); `countWords("guarda-chuva")` = 1; `countWords("")` = 0.
  - `useAutosave`: três `schedule` dentro de 800 ms resultam em 1 chamada de `save` com o último valor; `flush()` com algo pendente chama `save` imediatamente; `flush()` sem nada pendente não chama.
- [ ] **Step 2:** `npx vitest run`. Esperado: FAIL.
- [ ] **Step 3:** Implementar `wordcount`, `useAutosave` e `Typewriter` (`@tiptap/vue-3` com StarterKit, sem barra de ferramentas; atalhos de itálico e negrito). **Rolagem de máquina de escrever:** a cada transação, pegar `view.coordsAtPos(selection.head)` e rolar o container para deixar esse `y` no centro da tela. **Foco:** classe CSS que deixa todos os parágrafos com opacidade 0.3, menos o que contém o cursor. **Som de tecla:** clique curto via WebAudio, só com `keysound=on`. Largura máxima do texto: 68ch.
- [ ] **Step 4:** `EditorScreen`: sumário à esquerda (recolhível com `Ctrl/Cmd+\`), editor no centro, título editável da cena, botão "Versões" (lista com data e contagem de palavras, e "Restaurar"). Ao trocar de cena, trocar de livro ou ao fechar a janela (`getCurrentWindow().onCloseRequested`), chamar `await flush()` antes de continuar.
- [ ] **Step 5:** `npx vitest run`. Esperado: PASS.
- [ ] **Step 6: Verificação manual:** digitar, fechar o app imediatamente, reabrir → o texto está lá. Linha atual fica no meio da tela. Temas e fontes trocam. Restaurar uma versão funciona.
- [ ] **Step 7:** Commit `feat: editor máquina de escrever com autosave e versões`.

### Task 7: Exportar `.md` e `.docx`

**Files:**
- Create: `src/lib/export/markdown.ts`, `src/lib/export/docx.ts`
- Modify: `src/features/editor/EditorScreen.vue` (menu "Exportar")
- Test: `src/lib/export/markdown.test.ts`, `src/lib/export/docx.test.ts`

**Interfaces:**
- Consumes: `bookOutline`, `sceneGet`.
- Produces:
  ```ts
  type ExportBook = { title: string; chapters: { title: string; scenes: { title: string; doc: JSONContent }[] }[] };
  function toMarkdown(b: ExportBook): string;
  async function toDocx(b: ExportBook): Promise<Uint8Array>;
  ```
  Formato: `# título do livro`, `## capítulo`, cenas separadas por `* * *` (o título da cena não é exportado, já que é só para organização), itálico como `*x*`, negrito como `**x**`.

- [ ] **Step 1: Testes que falham**
  - Parágrafo `— Você viu? *Não*, disse ela. #1 e 2_3` (com `*` e `#` literais, sem marcação) → markdown com `\*`, `\#` e `\_` escapados, e o travessão e os acentos intactos.
  - Um trecho em itálico vira `*x*`.
  - Duas cenas num capítulo → separadas por `\n\n* * *\n\n`.
  - Capítulo sem cenas → só o `##` aparece.
  - `toDocx`: abrir o resultado com JSZip; `word/document.xml` contém "— Você viu?" e o título do capítulo.
- [ ] **Step 2:** `npx vitest run export`. Esperado: FAIL.
- [ ] **Step 3:** Implementar. Para `.docx`, usar o pacote `docx`: título como Heading1, capítulo como Heading2 com quebra de página antes, parágrafos com recuo de primeira linha, separador `* * *` centralizado. No menu "Exportar" → escolher o formato → `save()` do plugin dialog (nome sugerido: `<título>.md` ou `.docx`) → `writeFile` do plugin fs.
- [ ] **Step 4:** `npx vitest run`. Esperado: PASS.
- [ ] **Step 5: Verificação manual:** exportar um livro de 3 capítulos e abrir o `.docx` no Word ou no Pages.
- [ ] **Step 6:** Commit `feat: exportar livro para markdown e docx`.

### Task 8: Empacotamento

**Files:**
- Modify: `src-tauri/tauri.conf.json` (nome do produto, identificador, ícone)

- [ ] **Step 1:** Configurar `productName`, `identifier` e ícone.
- [ ] **Step 2:** `npm run tauri build` no macOS. Esperado: `.dmg` gerado em `src-tauri/target/release/bundle/`.
- [ ] **Step 3:** Instalar o `.dmg` e repetir a verificação manual da Task 4 e da Task 6 no app instalado.
- [ ] **Step 4:** Anotar no README como gerar o build no Windows (`npm run tauri build` numa máquina Windows, com Rust e o WebView2 instalados).
- [ ] **Step 5:** Commit `chore: empacotamento do app desktop`.
