# Lume

Uma máquina de escrever para livros: cofre cifrado no próprio computador, livro feito de cenas que se arrastam,
editor em que a linha atual fica sempre no meio da tela, e exportação para Word e Markdown.

Design: [docs/superpowers/specs/2026-11-17-maquina-de-escrever-design.md](docs/superpowers/specs/2026-11-17-maquina-de-escrever-design.md)

## Instalar

Os instaladores saem do GitHub, na página de **Releases** (cada versão `vX.Y.Z` gera todos sozinha).

| Sistema | Arquivo |
| --- | --- |
| Windows | `Lume_<versão>_x64-setup.exe` (ou o `.msi`) |
| macOS com chip Apple (M1 ou mais novo) | `Lume_<versão>_aarch64.dmg` |
| macOS (Intel) | `Lume_<versão>_x64.dmg` |
| Linux (Debian, Ubuntu, Mint) | `Lume_<versão>_amd64.deb` |
| Linux (outras distribuições) | `Lume_<versão>_amd64.AppImage` (dê permissão: `chmod +x`) |

Os instaladores ainda não têm a assinatura paga da Apple e da Microsoft, então o sistema avisa:

- **macOS, jeito mais fácil (sem aviso):** rode no Terminal:

  ```bash
  curl -fsSL https://raw.githubusercontent.com/phs-santos/Lume/main/scripts/install-macos.sh | bash
  ```

  Ele baixa a versão certa para o seu Mac, confere a soma SHA-256 e põe o Lume em Aplicativos. O mesmo comando atualiza.
- **macOS, pelo `.dmg`:** arraste o Lume para Aplicativos e abra. No aviso, vá em **Ajustes do Sistema → Privacidade
  e Segurança → Abrir Mesmo Assim**. Ou, no Terminal: `xattr -dr com.apple.quarantine /Applications/Lume.app`.
- **Windows:** no aviso do SmartScreen, clique em **Mais informações → Executar assim mesmo**.

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

Para lançar uma versão, no Claude Code: `/lancar-versao` (ou `/lancar-versao patch`, `minor`, `major`, `rascunho`).
O agente `notas-da-versao` junta o que mudou desde a última versão e escreve as notas; o agente `lancador` roda os
testes, sobe a versão (`scripts/bump-version.sh`), cria a tag, acompanha o build no GitHub Actions e publica.

À mão: `scripts/bump-version.sh minor`, atualize o topo de `.github/release-notes.md`, faça o commit, crie a tag e
envie (`git tag v0.2.0 && git push origin main v0.2.0`). O GitHub gera tudo num release em rascunho para publicar.

Localmente, no macOS (gera `.dmg` em `src-tauri/target/release/bundle/dmg/`):

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
