# Plano 2 — Captura no celular e caixa de entrada

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ela grava (ou digita) uma ideia no celular em `lume.codexolimpo.com`; a ideia viaja cifrada pela VPS e aparece decifrada na caixa de entrada do Lume no computador, onde vira cena ou é arquivada.

**Architecture:** Três peças. (1) `server/`: Node 24 + Hono + `node:sqlite`, contas e envelopes cifrados, nunca vê conteúdo. (2) `capture/`: PWA Vue que deriva o segredo de login com Argon2id e cifra com `crypto_box_seal` (libsodium) para a chave pública da conta. (3) Desktop: módulo Rust `remote` que conecta a conta, gera o par X25519, baixa, decifra (crate `crypto_box`, compatível com libsodium), guarda na tabela local `inbox` e confirma; tela "Ideias" no editor.

**Tech Stack:** Node 24, Hono 4, `@hono/node-server`, `node:sqlite`, Vitest; Vue 3 + Vite, `libsodium-wrappers-sumo`, `idb-keyval`; Rust `crypto_box` 0.9 (feature `seal`), `reqwest` (rustls, blocking), `sha2`; Docker + Caddy na VPS.

**Spec:** `docs/superpowers/specs/2026-11-17-maquina-de-escrever-design.md` (seções 6, 6.2, 6.4, 7.2 e 11)

## Global Constraints

- Endereço público: `https://lume.codexolimpo.com`; contêiner em `127.0.0.1:3020`; código em `/root/lume`; Caddy em `/etc/caddy/sites/lume.caddy`.
- O servidor nunca recebe senha, chave privada, áudio ou texto em claro.
- Segredo de login: Argon2id, m=65536 KiB, t=3, p=1, sal de 16 bytes, saída 32 bytes (igual nos dois lados; libsodium `crypto_pwhash` com `OPSLIMIT=3`, `MEMLIMIT=67108864`, `ALG_ARGON2ID13`).
- Servidor guarda `SHA-256(segredo_de_login)` em hex.
- Cifra dos envelopes: `crypto_box_seal` (X25519 + XSalsa20-Poly1305).
- Cadastro só com `INVITE_CODE` correto.
- Envelope máximo: 25 MB. Sessão expira em 90 dias.
- Envelopes apagados no servidor assim que o desktop confirma.
- Nenhuma alteração na VPS sem avisar o usuário antes (Task 7).
- Interface em português do Brasil, mesmo visual do Lume (cores e fontes de `src/styles/themes.css`).

## Review Focus

1. **Celular sem rede ao gravar:** o envelope fica na fila do aparelho e é enviado sozinho quando a rede volta; nada se perde. → teste em Task 4.
2. **Senha errada ou apelido inexistente no login:** mesma mensagem ("Apelido ou senha incorretos"), sem revelar qual errou; muitas tentativas → 429. → teste em Task 1.
3. **Uma conta tentando ler ou apagar envelope de outra:** 404. → teste em Task 2.
4. **Desktop cai no meio da sincronização:** um envelope só é apagado no servidor depois de gravado no banco local; repetir a sincronização não duplica. → teste em Task 5.
5. **Áudio do iPhone (mp4/aac) e do Android (webm/opus):** o tipo viaja junto e o desktop toca os dois. → teste em Task 4 (tipo registrado) e Task 6 (blob com o tipo certo).

---

## Estrutura de arquivos

```
server/
  package.json  tsconfig.json  Dockerfile  docker-compose.yml  lume.caddy
  src/db.ts         # abre SQLite + migrações
  src/accounts.ts   # cadastro, sal, login, sessões, limite de tentativas
  src/inbox.ts      # envelopes
  src/app.ts        # rotas Hono (createApp(db, opts))
  src/index.ts      # sobe o servidor, serve capture/dist
  test/*.test.ts
capture/
  package.json  vite.config.ts  index.html  public/manifest.webmanifest  public/icon-*.png
  src/crypto.ts     # deriveLoginSecret, seal (libsodium)
  src/queue.ts      # fila offline (IndexedDB) + envio com nova tentativa
  src/api.ts        # chamadas ao servidor
  src/App.vue  src/screens/{Login,Capture}.vue  src/sw.ts
  scripts/make-fixtures.mjs   # gera test-fixtures/crypto.json com libsodium
  test/*.test.ts
test-fixtures/crypto.json     # usado pelo teste de interoperabilidade em Rust
src-tauri/src/remote/{mod.rs,crypto.rs,client.rs,sync.rs}
src-tauri/src/db/inbox.rs     # tabela local inbox (migração v2)
src/features/inbox/{InboxPanel.vue,ConnectPhone.vue}
```

---

### Task 1: Servidor — contas e sessões

**Files:** Create `server/package.json`, `server/tsconfig.json`, `server/src/{db.ts,accounts.ts,app.ts}`; Test `server/test/accounts.test.ts`

**Interfaces — Produces:**
```ts
// db.ts
export function openDb(path: string): DatabaseSync;   // ":memory:" nos testes; migração v1
// tabelas: accounts(id TEXT PK, nickname TEXT UNIQUE COLLATE NOCASE, salt BLOB, secret_hash TEXT, public_key BLOB NULL, created_at INT)
//          sessions(token_hash TEXT PK, account_id TEXT, created_at INT, expires_at INT)
//          envelopes(id TEXT PK, account_id TEXT, kind TEXT, mime TEXT, body BLOB, created_at INT)
// app.ts
export function createApp(db: DatabaseSync, opts: { inviteCode: string; now?: () => number }): Hono;
```
Rotas:
- `POST /api/accounts` `{nickname, invite, salt(b64,16B), secret(b64,32B)}` → 201 `{token}`; convite errado 403; apelido em uso 409.
- `GET /api/accounts/:nickname/salt` → 200 `{salt}`. Apelido inexistente → 200 com sal **determinístico falso** (`HMAC-SHA256(INVITE_CODE, nickname)[0..16]`), para não revelar quem existe.
- `POST /api/sessions` `{nickname, secret}` → 200 `{token}` | 401 `{error:"Apelido ou senha incorretos"}`; mais de 10 falhas por apelido em 15 min → 429.
- `PUT /api/accounts/me/public-key` (Bearer) `{public_key(b64,32B)}` → 204. `GET /api/accounts/me/public-key` → `{public_key}` ou 404.
- Token: 32 bytes aleatórios em base64url; servidor guarda só `SHA-256(token)`.

- [ ] **Step 1:** Criar `server/` (`npm init`, `hono`, `@hono/node-server`, `vitest`, `typescript`, `tsx`), `"type": "module"`.
- [ ] **Step 2: Testes que falham** (`accounts.test.ts`, usando `app.request(...)`):
  - cadastro com convite certo → 201 e token; com convite errado → 403; apelido repetido (outra caixa: "Henri" vs "henri") → 409
  - sal de conta existente = sal cadastrado; sal de apelido inexistente é estável entre chamadas e tem 16 bytes
  - login certo → token; segredo errado → 401 com a mensagem; apelido inexistente → 401 com **a mesma** mensagem
  - 11ª tentativa errada em 15 min → 429; depois de 15 min (relógio injetado) volta a aceitar
  - `PUT/GET public-key` sem token → 401; com token → grava e lê
  - o banco não contém o segredo em claro (só o hash)
- [ ] **Step 3:** `npx vitest run` em `server/`. Esperado: FAIL.
- [ ] **Step 4:** Implementar `db.ts`, `accounts.ts`, `app.ts`.
- [ ] **Step 5:** `npx vitest run`. Esperado: PASS.
- [ ] **Step 6:** Commit `feat(server): contas, sessões e limite de tentativas`.

### Task 2: Servidor — envelopes

**Files:** Create `server/src/inbox.ts`; Modify `server/src/app.ts`; Test `server/test/inbox.test.ts`

**Interfaces — Produces:**
- `POST /api/inbox` (Bearer), corpo binário, cabeçalhos `X-Lume-Kind: audio|text`, `Content-Type: <mime original>` (ex.: `audio/webm;codecs=opus`, `audio/mp4`, `text/plain`) → 201 `{id}`; >25 MB → 413; kind inválido → 400.
- `GET /api/inbox` → `[{id, kind, mime, size, created_at}]` mais antigos primeiro.
- `GET /api/inbox/:id` → corpo binário (`application/octet-stream`), cabeçalhos `X-Lume-Kind`, `X-Lume-Mime`.
- `DELETE /api/inbox/:id` → 204; de outra conta ou inexistente → 404.
- `GET /api/health` → `{ok:true}`.

- [ ] **Step 1: Testes que falham:** postar e listar; baixar devolve os mesmos bytes; apagar some da lista; conta B não lista, não baixa e não apaga envelope da conta A (404); 25 MB + 1 → 413; sem token → 401; `kind` "video" → 400; mime é devolvido igual.
- [ ] **Step 2:** `npx vitest run`. Esperado: FAIL.
- [ ] **Step 3:** Implementar. Limite de corpo com `hono/body-limit`.
- [ ] **Step 4:** `npx vitest run`. Esperado: PASS.
- [ ] **Step 5:** Commit `feat(server): caixa de entrada de envelopes cifrados`.

### Task 3: Cripto compartilhada e prova de interoperabilidade

**Files:** Create `capture/package.json`, `capture/src/crypto.ts`, `capture/scripts/make-fixtures.mjs`, `test-fixtures/crypto.json`, `src-tauri/src/remote/{mod.rs,crypto.rs}`; Modify `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`; Test `capture/test/crypto.test.ts`, `#[cfg(test)]` em `remote/crypto.rs`

**Interfaces — Produces:**
```ts
// capture/src/crypto.ts
export async function deriveLoginSecret(password: string, salt: Uint8Array): Promise<Uint8Array>; // 32 B
export async function seal(message: Uint8Array, publicKey: Uint8Array): Promise<Uint8Array>;
export const b64: { encode(b: Uint8Array): string; decode(s: string): Uint8Array }; // base64 padrão
```
```rust
// remote/crypto.rs
pub fn derive_login_secret(password: &str, salt: &[u8]) -> Result<[u8; 32], VaultError>;
pub struct Keypair { pub public: [u8; 32], pub secret: [u8; 32] }   // secret zerado no drop
pub fn generate_keypair() -> Keypair;
pub fn seal_open(sealed: &[u8], kp: &Keypair) -> Option<Vec<u8>>;
```
`test-fixtures/crypto.json`: `{ password, salt, login_secret, secret_key, public_key, plaintext, sealed }` (base64), gerado pelo `make-fixtures.mjs` com libsodium.

- [ ] **Step 1:** Criar `capture/` com Vite + Vue + TS; instalar `libsodium-wrappers-sumo`, `vitest`. Escrever `make-fixtures.mjs` e rodar: `node scripts/make-fixtures.mjs > ../test-fixtures/crypto.json`.
- [ ] **Step 2: Testes que falham:** TS — `deriveLoginSecret(fixture.password, fixture.salt)` = `fixture.login_secret`; `seal` + `crypto_box_seal_open` (libsodium) devolve a mensagem. Rust — `derive_login_secret` = `fixture.login_secret`; `seal_open(fixture.sealed, kp_da_fixture)` = `fixture.plaintext`; envelope adulterado (um byte trocado) → `None`; `generate_keypair` produz pares que abrem o que o próprio `crypto_box` sela.
- [ ] **Step 3:** Rodar os dois. Esperado: FAIL.
- [ ] **Step 4:** Implementar (`crypto_box = { version = "0.9", features = ["seal"] }`; Argon2 reaproveita os parâmetros de `vault.rs`).
- [ ] **Step 5:** `npx vitest run` (capture) e `cargo test remote::crypto`. Esperado: PASS.
- [ ] **Step 6:** Commit `feat: cripto compatível entre celular (libsodium) e desktop (Rust)`.

### Task 4: Página de captura (PWA)

**Files:** Create `capture/src/{api.ts,queue.ts,App.vue,main.ts,sw.ts}`, `capture/src/screens/{Login.vue,Capture.vue}`, `capture/public/manifest.webmanifest`, ícones; Test `capture/test/queue.test.ts`, `capture/test/api.test.ts`

**Interfaces — Produces:**
```ts
// queue.ts — fila em IndexedDB (idb-keyval); itens já cifrados
export interface Pending { id: string; kind: "audio" | "text"; mime: string; body: Uint8Array; createdAt: number; attempts: number }
export async function enqueue(p: Omit<Pending, "id" | "attempts">): Promise<Pending>;
export async function pending(): Promise<Pending[]>;
export async function flush(send: (p: Pending) => Promise<void>): Promise<{ sent: number; left: number }>;
// api.ts
export async function login(nickname: string, password: string): Promise<{ token: string; publicKey: Uint8Array }>;
export async function postEnvelope(token: string, p: Pending): Promise<void>;
```
Telas: **Login** (apelido, senha, botão "Entrar"; mesmo cartão de papel do Lume, cena da escrivaninha de fundo simplificada). **Capture**: botão redondo grande "Gravar" (vermelho da fita) que vira "Parar" com cronômetro; campo "Ou escreva uma ideia" + "Enviar"; linha de estado "3 ideias esperando rede" / "Tudo enviado"; botão "Sair". Token e chave pública ficam no `localStorage`; a senha nunca é guardada.

- [ ] **Step 1: Testes que falham** (`fake-indexeddb`):
  - `enqueue` + `pending` devolve o item; `flush` com envio OK esvazia a fila
  - `flush` com envio falhando mantém o item e incrementa `attempts`; um segundo `flush` com rede OK envia
  - itens saem na ordem de criação
  - `postEnvelope` manda `X-Lume-Kind` e o `Content-Type` com o mime original (`audio/mp4` e `audio/webm;codecs=opus`) — `fetch` substituído por um falso que registra a requisição
  - `login` com 401 lança erro com a mensagem do servidor
- [ ] **Step 2:** `npx vitest run`. Esperado: FAIL.
- [ ] **Step 3:** Implementar. Gravação: `MediaRecorder` com o primeiro tipo suportado entre `audio/webm;codecs=opus` e `audio/mp4`. Ao parar: cifra → `enqueue` → `flush`. `flush` também roda em `online` e a cada 30 s. `sw.ts`: cache do app para abrir sem rede.
- [ ] **Step 4:** `npx vitest run` e `npm run build`. Esperado: PASS, `dist/` gerado.
- [ ] **Step 5: Verificação manual** no navegador embutido com o servidor local (`server` em `:3020` servindo `capture/dist`): cadastrar via `curl` com convite, entrar, mandar uma ideia de texto, ver o envelope no `GET /api/inbox`.
- [ ] **Step 6:** Commit `feat(capture): página de captura com fila offline`.

### Task 5: Desktop — conectar conta e sincronizar

**Files:** Create `src-tauri/src/remote/{client.rs,sync.rs}`, `src-tauri/src/db/inbox.rs`; Modify `src-tauri/src/db/mod.rs` (migração v2), `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src/lib/api.ts`, `src/lib/types.ts`

**Interfaces — Produces:**
```rust
// migração v2
// inbox(id TEXT PK /* = id do envelope */, kind TEXT, mime TEXT, body BLOB, created_at INT, received_at INT, status TEXT CHECK(status IN('nova','arquivada')))
pub trait InboxApi {
    fn list(&self) -> Result<Vec<EnvelopeMeta>, RemoteError>;
    fn fetch(&self, id: &str) -> Result<Vec<u8>, RemoteError>;
    fn delete(&self, id: &str) -> Result<(), RemoteError>;
}
pub struct HttpApi { base: String, token: String }               // reqwest blocking
pub fn sync(c: &Connection, api: &dyn InboxApi, kp: &Keypair, now: i64) -> Result<usize, RemoteError>;
// settings: remote_url, remote_nickname, remote_token, remote_public, remote_secret (b64; tudo dentro do banco cifrado)
```
Comandos: `remote_status() -> { connected, nickname, url }`, `remote_connect(url, nickname, password, invite: Option<String>)` (sem conta → cadastra com convite; com conta → login; gera par se não houver e faz `PUT public-key`), `remote_disconnect()`, `remote_sync() -> usize`, `inbox_list() -> Vec<InboxItem>` (sem o corpo), `inbox_body(id) -> Vec<u8>`, `inbox_archive(id)`, `inbox_delete(id)`, `inbox_to_scene(id, chapter_id, index: Option<i64>) -> Scene` (só `kind = text`: o texto vira o conteúdo da cena, um parágrafo por linha).

- [ ] **Step 1: Testes que falham** (`sync.rs` com um `FakeApi` em memória):
  - 2 envelopes válidos → 2 itens `nova` no banco e 2 `delete` no fake
  - repetir `sync` com o mesmo envelope ainda no servidor (o delete falhou antes) não duplica e tenta apagar de novo
  - `fetch` falhando no 2º → o 1º fica salvo e apagado; o 2º continua no servidor; retorna erro só depois de processar o resto
  - envelope que não abre com a chave → não é apagado, não entra no banco, é contado como erro
  - `inbox_to_scene` com texto de 3 linhas cria cena com 3 parágrafos e marca o item como `arquivada`; com áudio → erro `NOT_TEXT`
  - migração v2 roda em banco v1 existente sem perder livros
- [ ] **Step 2:** `cargo test`. Esperado: FAIL.
- [ ] **Step 3:** Implementar. `remote_connect` busca o sal (`GET .../salt`); se o apelido não existe no servidor e veio convite, gera sal novo e cadastra. Comandos de rede como `#[tauri::command(async)]`.
- [ ] **Step 4:** `cargo test`. Esperado: PASS.
- [ ] **Step 5:** Commit `feat(desktop): conectar ao celular e sincronizar a caixa de entrada`.

### Task 6: Desktop — tela de Ideias

**Files:** Create `src/features/inbox/{InboxPanel.vue,ConnectPhone.vue}`, `src/features/inbox/audioUrl.ts`; Modify `src/features/editor/EditorScreen.vue`, `src/features/editor/shortcuts.ts`, `src/dev/mockBackend.ts`; Test `src/features/inbox/audioUrl.test.ts`, `src/features/editor/shortcuts.test.ts`

**Interfaces — Produces:**
```ts
export function audioUrl(body: Uint8Array, mime: string): string; // Blob com o mime certo → object URL
```
- Botão "Ideias" na barra do editor com contador de novas; atalho `⌘ ⇧ I`.
- Painel: cartões com data/hora; áudio com player; texto com o conteúdo; ações "Virar cena aqui" (texto; cria depois da cena atual), "Arquivar", "Apagar" (confirma). Abas "Novas" / "Arquivadas".
- Sem conta conectada: o painel mostra **ConnectPhone**: endereço (padrão `https://lume.codexolimpo.com`), apelido (o do cofre), senha, convite (só na primeira vez) e um texto curto explicando que a senha é a mesma do cofre.
- Sincroniza ao abrir o livro, a cada 60 s e no botão "Buscar agora".

- [ ] **Step 1: Testes que falham:** `audioUrl` gera blob com o mime passado (`audio/mp4` e `audio/webm`); `matchShortcut(⌘⇧I)` → `"ideas"`.
- [ ] **Step 2:** `npx vitest run`. Esperado: FAIL.
- [ ] **Step 3:** Implementar; mock do backend ganha `remote_*`/`inbox_*` em memória para revisar no navegador.
- [ ] **Step 4:** `npx vitest run` e `npx vue-tsc --noEmit`. Esperado: PASS.
- [ ] **Step 5: Verificação manual** no navegador com o mock: painel vazio, conectar, ver cartões, virar cena.
- [ ] **Step 6:** Commit `feat(desktop): painel de ideias`.

### Task 7: Implantação na VPS

**Files:** Create `server/Dockerfile`, `server/docker-compose.yml`, `server/lume.caddy`, `server/src/index.ts`, `scripts/deploy.sh`

**Interfaces — Consumes:** `createApp` (Task 1–2), `capture/dist` (Task 4).

- [ ] **Step 1:** `index.ts`: lê `PORT` (3020), `DB_PATH` (`/data/lume.db`), `INVITE_CODE` (obrigatório; sem ele o processo encerra com mensagem clara), serve `capture/dist` em `/` e a API em `/api`.
- [ ] **Step 2:** `Dockerfile` multi-stage (`node:24-slim`): build do `capture` e do `server`; usuário não-root. `docker-compose.yml`: serviço `lume`, `ports: ["127.0.0.1:3020:3020"]`, volume `lume-data:/data`, `env_file: .env`, `restart: unless-stopped`. `lume.caddy`: `lume.codexolimpo.com { encode gzip; reverse_proxy 127.0.0.1:3020 }`.
- [ ] **Step 3:** Local: `docker compose up --build` com `.env` de teste → `curl localhost:3020/api/health` = `{"ok":true}`.
- [ ] **Step 4: Avisar o usuário e esperar o "ok"** antes de qualquer mudança na VPS.
- [ ] **Step 5:** `scripts/deploy.sh`: `rsync` de `server/`, `capture/`, `test-fixtures/` para `vps:/root/lume`; gerar `INVITE_CODE` aleatório em `/root/lume/server/.env` (só se ainda não existir); `docker compose up -d --build`; copiar `lume.caddy` para `/etc/caddy/sites/`, `caddy validate` e `systemctl reload caddy`.
- [ ] **Step 6:** Verificar `curl https://lume.codexolimpo.com/api/health` = `{"ok":true}` e que os outros sites continuam respondendo (`viagens`, `logfarma`).
- [ ] **Step 7:** Commit `chore: implantação do servidor do Lume`.
