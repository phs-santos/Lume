# Imagens no texto e EPUB — Plano

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Inserir imagens com legenda no texto (guardadas cifradas), preencher a ficha do livro (autor, sinopse, dedicatória, capa, contracapa) e exportar um EPUB 3 bonito que abre no Kindle.

**Architecture:** Rust guarda imagens e a ficha no banco SQLCipher (migração 5) e as entrega ao front por comandos (bytes crus de volta, base64 de ida). O editor ganha um nó TipTap `figure` (`src = "lume-img:<id>"`, `caption`). Capa gerada e EPUB são montados no front, como as outras exportações (JSZip).

**Tech Stack:** Rust/rusqlite; Vue 3 + TipTap 2; JSZip; Canvas 2D; Vitest (happy-dom).

**Spec:** `docs/superpowers/specs/2026-11-18-imagens-e-epub-design.md`

## Global Constraints

- Referência de imagem no documento: `lume-img:<uuid>`; nada de caminho de arquivo nem `data:` no JSON da cena.
- Ao inserir: lado maior ≤ 1600 px; JPEG qualidade 0,85, ou PNG se houver transparência; recusar arquivo > 10 MB ("Imagem grande demais (máx. 10 MB).").
- Capa gerada: 1600 × 2560 px, PNG.
- EPUB 3: `mimetype` primeiro e sem compressão; `nav.xhtml` + `toc.ncx`; `dc:title`, `dc:creator`, `dc:language = pt-BR`, `dc:identifier` estável por livro (`urn:uuid:<book id>`), `meta property="dcterms:modified"`; capa com `properties="cover-image"`.
- Ordem: capa · rosto · dedicatória (se houver) · sumário · capítulos · contracapa (se houver sinopse ou imagem).
- Word, PDF e Markdown: a imagem vira `[Imagem: legenda]` (ou `[Imagem]`).
- Textos da interface em português.

## Review Focus

1. Imagem colada **no meio** de um parágrafo: o nó entra como bloco e o parágrafo não se perde. → teste Task 2.
2. Restaurar versão antiga com imagem que foi apagada do texto depois: a imagem ainda existe (a limpeza respeita versões). → teste Task 1.
3. Título/legenda com `&`, `<`, aspas e acentos: XHTML do EPUB continua bem formado. → teste Task 4.
4. Livro sem autor, sem sinopse, sem capa: EPUB válido mesmo assim (capa gerada, sem contracapa). → teste Task 4.
5. Arquivo de imagem corrompido ou grande demais: mensagem clara, nada é gravado. → teste Task 2.

---

### Task 1: Banco — imagens e ficha do livro

**Files:** Modify `src-tauri/src/db/mod.rs` (migração 5), Create `src-tauri/src/db/images.rs`, Modify `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src/lib/api.ts`, `src/lib/types.ts`, `src/dev/mockBackend.ts`

**Interfaces — Produces:**
```rust
// migração 5:
// images(id TEXT PK, book_id TEXT NOT NULL, mime TEXT NOT NULL, bytes BLOB NOT NULL, width INT, height INT, created_at INT)
// ALTER TABLE books ADD author TEXT; synopsis TEXT; dedication TEXT; cover_image_id TEXT; back_image_id TEXT
pub fn image_add(c, book_id, mime, bytes: &[u8], width: i64, height: i64, now) -> Result<String, DbError>;
pub fn image_get(c, id) -> Result<(String /*mime*/, Vec<u8>), DbError>;
pub fn images_gc(c, book_id) -> Result<usize, DbError>; // apaga as que não aparecem em scenes, scene_versions nem na ficha
pub struct BookMeta { author, synopsis, dedication, cover_image_id, back_image_id: Option<String> }
pub fn book_meta_get(c, book_id) -> Result<BookMeta, DbError>;
pub fn book_meta_set(c, book_id, &BookMeta) -> Result<(), DbError>;
```
Comandos: `image_add(bookId, mime, b64, width, height) -> id`, `image_get(id) -> ipc::Response (bytes)` com o mime no cabeçalho via `image_mime(id)`, `book_meta_get`, `book_meta_set`, `images_gc(bookId)`.

- [ ] Testes que falham: add/get ida e volta; gc apaga imagem sem referência; gc mantém imagem referenciada só numa versão antiga; gc mantém capa da ficha; meta get/set; livro sem ficha devolve tudo `None`; migração 5 roda num banco v4.
- [ ] Implementar; `cargo test` verde; commit `feat(imagens): banco de imagens e ficha do livro`.

### Task 2: Imagem no editor

**Files:** Create `src/features/editor/figure.ts`, `src/features/editor/imageFile.ts`; Modify `src/features/editor/editorConfig.ts`, `Typewriter.vue`, `EditorScreen.vue`, exports `markdown.ts`/`docx.ts`/`pdf.ts`; Tests `imageFile.test.ts`, `figure.test.ts`, export tests.

**Interfaces — Produces:**
```ts
export function fitWithin(w: number, h: number, max = 1600): { width: number; height: number };
export async function prepareImage(file: Blob): Promise<{ mime: string; bytes: Uint8Array; width: number; height: number }>; // canvas
export const Figure: Node; // name "figure", atom, block, attrs { src, caption, width, height }
export function imageUrl(id: string): Promise<string>; // cache de blob URLs por id
// Typewriter expõe insertImage(id, w, h)
```
- [ ] Testes que falham: `fitWithin` (3000×2000 → 1600×1067; 800×600 igual); `prepareImage` recusa > 10 MB e tipo não imagem; inserir figure no meio de um parágrafo divide o parágrafo e mantém o texto; JSON da cena guarda `lume-img:<id>`; exports viram `[Imagem: legenda]`.
- [ ] Implementar: colar/arrastar imagem chama `prepareImage` → `imageAdd` → `insertImage`; botão "Imagem" no menu da cena; NodeView mostra a imagem e a legenda editável.
- [ ] Verificação no navegador (mock). Commit `feat(imagens): imagens com legenda no editor`.

### Task 3: Ficha do livro e capa gerada

**Files:** Create `src/features/book/BookSheet.vue`, `src/features/book/cover.ts`; Modify `EditorScreen.vue` (acesso pela barra ou menu do livro); Tests `cover.test.ts`, `BookSheet.test.ts`.

**Interfaces — Produces:**
```ts
export function wrapTitle(title: string, maxChars: number): string[];      // quebra em linhas equilibradas
export async function generateCover(o: { title: string; author: string; color: string }): Promise<Uint8Array>; // PNG 1600×2560
```
- [ ] Testes que falham: `wrapTitle` equilibrado e sem cortar palavra; `BookSheet` salva autor/sinopse/dedicatória e escolhe capa (imagem ou gerada).
- [ ] Implementar; commit `feat(livro): ficha do livro e capa gerada`.

### Task 4: Exportar EPUB

**Files:** Create `src/lib/export/epub.ts`; Modify `EditorScreen.vue` (opção EPUB no Exportar); Test `epub.test.ts`.

**Interfaces — Produces:**
```ts
export type EpubInput = ExportBook & { id: string; author?: string; synopsis?: string; dedication?: string;
  cover: { mime: string; bytes: Uint8Array }; back?: { mime: string; bytes: Uint8Array };
  images: Map<string, { mime: string; bytes: Uint8Array }> };
export async function toEpub(b: EpubInput): Promise<Uint8Array>;
```
- [ ] Testes que falham: `mimetype` é a primeira entrada e STORE; OPF tem título, autor, idioma, identificador, `dcterms:modified`, capa `cover-image` e cada imagem no manifest; spine na ordem do design; `nav.xhtml` e `toc.ncx` com um item por capítulo; todo XHTML é XML bem formado (DOMParser) mesmo com `&`, `<`, aspas; imagens do texto viram `<figure><img/><figcaption/></figure>` apontando para o arquivo certo; livro sem autor/sinopse funciona e não tem contracapa.
- [ ] Implementar; commit `feat(exportar): EPUB com capa, sumário e imagens`.

### Task 5: Validação e entrega

- [ ] Rodar `epubcheck` (instalar Java/epubcheck se o usuário aprovar) num EPUB gerado de "Até ter certeza" com imagens de teste. Esperado: sem erros.
- [ ] Abrir o EPUB no app Livros do Mac e conferir capa, sumário, imagens e contracapa.
- [ ] Todas as suítes; build; instalar; commit.
