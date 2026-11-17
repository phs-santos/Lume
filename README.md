# Lume

Uma máquina de escrever para livros: cofre cifrado no próprio computador, livro feito de cenas que se arrastam,
editor em que a linha atual fica sempre no meio da tela, e exportação para Word e Markdown.

Design: [docs/superpowers/specs/2026-11-17-maquina-de-escrever-design.md](docs/superpowers/specs/2026-11-17-maquina-de-escrever-design.md)

## Desenvolvimento

Precisa de Node 20+ e Rust (stable).

```bash
npm install
npm run tauri dev
```

Abrindo `http://localhost:1420` num navegador comum (fora do Tauri), a interface roda com um backend simulado em
memória (`src/dev/mockBackend.ts`). Isso serve só para mexer no visual; a senha de desbloqueio nesse modo é `senha123`.

## Testes

```bash
npm test
cargo test --manifest-path src-tauri/Cargo.toml
```

## Gerar o instalador

macOS (gera `.dmg` em `src-tauri/target/release/bundle/dmg/`):

```bash
npm run tauri build
```

Windows: rode o mesmo `npm run tauri build` numa máquina Windows com Node, Rust (toolchain MSVC) e o
WebView2 instalados (no Windows 10/11 atual o WebView2 já vem). O instalador `.msi`/`.exe` sai em
`src-tauri\target\release\bundle\`.

## Onde ficam os dados

Na pasta de dados do app (`~/Library/Application Support/com.lume.app/` no macOS,
`%APPDATA%\com.lume.app\` no Windows):

- `vault.json`: a chave do banco, embrulhada pela senha e pelas 24 palavras. Não contém texto do livro.
- `livro.db`: o banco SQLCipher com os livros, cifrado.

Sem a senha e sem as 24 palavras, ninguém consegue abrir esses arquivos.
