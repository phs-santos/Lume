# Máquina de Escrever — Design

*Data: 2026-11-17 · Status: rascunho para aprovação · Nome do app: a definir*

## 1. Intenção e o porquê

Um programa para duas pessoas: uma escritora (a namorada) e um escritor ocasional (o autor deste projeto).
Cada um escreve seus próprios livros no próprio computador.

O jeito natural dela de criar é **falando**: ela manda áudios, conversa com IAs para transcrever e
desenvolver ideias, e depois **copia e cola** o resultado num editor de texto e organiza na ordem do livro,
muitas vezes encaixando cenas no meio e reordenando. Esse copiar-colar-reorganizar é o atrito que o
programa elimina.

O programa é **o lugar onde o livro vive**, com a sensação de uma **máquina de escrever**: foco, tela
limpa, só o texto. Ideias faladas no celular chegam ao computador já transcritas e prontas para virar cena.

## 2. Objetivos

1. Editor de livro com sensação de máquina de escrever, onde o livro é uma **pilha de cenas** que se
   reordenam arrastando (cenas agrupadas em capítulos).
2. **Captura por voz no celular**: botão grande de gravar; o áudio chega cifrado ao computador da pessoa.
3. **Caixa de entrada de ideias** no computador: cada áudio vira um cartão com transcrição; dali vira cena
   nova, é anexado a uma cena existente, ou descartado.
4. **IA com dois modos por livro**:
   - **Conselheira** (o dela): conversa, sugere, aponta furos — **nunca escreve no manuscrito**.
   - **Coautora** (o dele): escreve rascunhos de cena como **propostas** que só entram no livro com aprovação.
5. **Privacidade forte**: a VPS só guarda dados cifrados que o administrador não consegue ler.
6. **Provedores de IA trocáveis** via adapter (Inworld, OpenRouter, outros).

## 3. Não-objetivos (v1)

- Colaboração ou compartilhamento de livros entre os dois.
- Editar o livro pelo celular (o celular só captura ideias).
- "Bíblia viva" (fichas automáticas de personagem, linha do tempo, alertas de continuidade) — candidata à v2.
- Publicação, formatação para gráfica, ePub.
- Contas para outras pessoas além dos dois.

## 4. Anti-objetivos (seria fracasso mesmo funcionando)

- A IA alterar uma palavra do texto dela no modo Conselheira.
- O servidor (ou o administrador) conseguir ler livros, áudios ou transcrições.
- Uma ideia gravada se perder no caminho.
- O livro se perder porque o notebook quebrou.
- Interface poluída que tira o foco da escrita.

## 5. Abordagens consideradas

| Abordagem | Trade-off | Decisão |
|---|---|---|
| **A) App desktop + página de captura (PWA) + caixa de entrada na VPS** | Sob medida dos dois lados, livro local; exige construir um servidor pequeno | **Escolhida** — o autor já tem uma VPS |
| B) Bot do Telegram como caixa de entrada | Rapidíssimo, sem servidor; mas o Telegram só guarda pendências ~24h → risco de perder ideias | Descartada (viola anti-objetivo) |
| C) Tudo web | Sincronização trivial; mas o livro mora no servidor e a imersão diminui | Descartada (contra "local" e privacidade) |

## 6. Arquitetura

```
 CELULAR (PWA)                    VPS (só envelopes lacrados)          COMPUTADOR (app desktop)
 ┌──────────────┐  áudio cifrado  ┌───────────────────────┐  busca   ┌──────────────────────────┐
 │ login        │ ──────────────► │ API da caixa de       │ ◄─────── │ decifra → transcreve (IA)│
 │ 🎙 gravar    │  (sealed box p/ │ entrada + blobs       │          │ → cartão na caixa de     │
 │ fila offline │   chave pública)│ cifrados + backup     │ ◄──────► │   entrada → cena no livro│
 └──────────────┘                 │ cifrado do livro      │  backup  │ banco local cifrado      │
                                  └───────────────────────┘          └────────────┬─────────────┘
                                                                                   │ adapter
                                                                       Inworld / OpenRouter / ...
```

### 6.1 Componentes

- **App desktop** — Tauri 2 (Rust) + Vue 3/TypeScript. Editor de texto com TipTap/ProseMirror (`@tiptap/vue-3`).
  Banco local SQLite cifrado (SQLCipher). Roda em macOS e Windows.
- **Servidor da caixa de entrada** — serviço pequeno (TypeScript com Bun/Node ou Go) + SQLite/Postgres,
  atrás de Caddy (HTTPS automático) na VPS. Guarda: contas, chaves públicas, chaves privadas
  *embrulhadas*, blobs de áudio cifrados, backups cifrados do livro.
- **PWA de captura** — servida pela VPS, instalável na tela inicial do celular. Gravação com
  `MediaRecorder`, cifra no aparelho, fila offline (IndexedDB) para quando não houver rede.

### 6.2 Modelo de criptografia

- No cadastro, o app desktop gera um par de chaves **X25519** da pessoa (libsodium).
- A senha passa por **Argon2id** e gera duas coisas separadas: um *segredo de login* (o servidor vê só
  um hash disso) e uma *chave de embrulho* (nunca sai do aparelho). O servidor guarda a chave privada
  **embrulhada** por essa chave.
- **Celular**: precisa só da **chave pública** para cifrar (`crypto_box_seal`). Ele nem consegue ler
  áudios antigos — se o celular for roubado, as ideias anteriores continuam seguras.
- **Computador**: desembrulha a chave privada com a senha e abre os áudios.
- **Livro local**: banco SQLCipher com chave derivada da senha.
- **Backup do livro**: o app envia periodicamente um snapshot cifrado para a VPS (resolve o
  anti-objetivo "notebook quebrou"). Restaurar exige a senha ou a chave de recuperação.

### 6.3 Recuperação de senha (a "brecha")

- No cadastro é gerada uma **chave de recuperação** (24 palavras), que a pessoa imprime e guarda.
- A chave privada fica embrulhada **duas vezes**: pela senha e pela chave de recuperação.
- Esqueceu a senha → digita as 24 palavras → define senha nova. O administrador **não** consegue
  recuperar sozinho; a brecha está na mão dela, não do servidor.
- Perdeu senha **e** chave de recuperação → os dados cifrados são irrecuperáveis (consequência aceita).

### 6.4 Limites de confiança (conhecidos e aceitos)

- O administrador serve o código da PWA e poderia, em tese, alterá-lo. Aceito: ele é o único admin e é
  de confiança.
- O provedor de IA processa áudio e texto para transcrever e sugerir. O administrador não vê, mas o
  provedor sim.

## 7. Funcionalidades em detalhe

### 7.1 Editor (a máquina de escrever)

- Tela cheia, texto centralizado, largura de linha confortável, fonte serifada ou monoespaçada (escolha
  da pessoa), tema "papel" claro e tema escuro.
- **Rolagem de máquina de escrever**: a linha atual fica fixa no meio da tela.
- **Modo foco**: esmaece tudo exceto o parágrafo atual. Som de tecla opcional (desligado por padrão).
- Barra lateral recolhível com o **sumário**: capítulos → cenas. Arrastar reordena cenas e move entre
  capítulos. Botão "inserir cena aqui" entre quaisquer duas cenas.
- Contagem de palavras por cena, capítulo e livro. Salvamento automático contínuo + histórico de versões
  por cena.
- Exportar: `.docx` e `.md` (livro inteiro na ordem atual).

### 7.2 Caixa de entrada de ideias

- Cada áudio chega como **cartão**: data, duração, player, transcrição **literal** (sem "melhorar").
- Ações: **virar cena nova** (escolhendo onde encaixar), **anexar a uma cena** (vai para as notas da
  cena, não para o texto), **conversar com a IA sobre isso**, **arquivar**.
- Ideias também podem ser digitadas direto no desktop.

### 7.3 IA

**Modo Conselheira** (garantido por código, não só pela interface):
- Painel lateral de conversa, com contexto do livro (cena atual, resumo, notas).
- A camada de IA **não tem permissão de escrita** no manuscrito: as respostas só podem ir para o painel
  e para notas. Não existe botão "inserir no texto".

**Modo Coautora**:
- Gera rascunhos de cena a partir de ideias/áudios e do contexto do livro.
- Cada rascunho é uma **proposta** com: aprovar (entra no livro), pedir outra versão (com instrução),
  descartar. Pode também propor edições mostradas como diff, que só se aplicam com aprovação.

O modo é configurado por livro na criação e pode ser trocado nas configurações do livro.

### 7.4 Adapter de IA

Duas interfaces independentes, cada uma com implementações trocáveis:

```ts
interface Transcriber { transcribe(audio: Blob, lang: "pt-BR"): Promise<string> }
interface ChatModel   { chat(messages: Msg[], opts?: Opts): AsyncIterable<string> }
```

Implementações iniciais: **Inworld** e **OpenRouter** (OpenAI-compatível). As chaves de API ficam no
banco local cifrado de cada pessoa. Cada pessoa escolhe seu provedor.

## 8. Fora do escopo do construtor (já decidido)

- Abordagem A; VPS do autor; Tauri + Vue 3; libsodium; SQLCipher; adapter com Inworld e OpenRouter;
  dois modos de IA; login nickname + senha; chave de recuperação de 24 palavras.

## 9. Deixado para o construtor

- Linguagem do servidor (Bun/TS ou Go) e banco (SQLite ou Postgres).
- Formato de áudio no celular (Opus/WebM; fallback para AAC no iOS Safari).
- Política de retenção na VPS (sugestão: apagar o áudio cifrado após o desktop confirmar recebimento;
  manter os últimos N backups do livro).
- Design visual fino dos temas.

## 10. Chamadas que eu fiz (para revisar)

- **Backup cifrado do livro na VPS**: não foi pedido, mas sem ele um notebook quebrado perde o livro.
- **Celular só com chave pública**: celular roubado não expõe ideias antigas.
- **Recuperação via chave de 24 palavras** em vez de recuperação pelo admin, para manter "nem eu posso ler".
- **Transcrição literal** no modo Conselheira, sem correções de estilo.
- **Exportação .docx/.md** já na v1: ela precisa conseguir tirar o livro do programa.
- **Bíblia viva fica para a v2**, para a v1 sair mais cedo.
- **Tauri + Vue 3**: um único código para macOS e Windows, app leve.

## 11. Plano 2 — detalhes aprovados (2026-11-17)

**Hospedagem (VPS existente, Ubuntu 24.04):**
- Endereço: `https://lume.codexolimpo.com` (DNS curinga já aponta para a VPS).
- Contêiner Docker `lume` em `127.0.0.1:3020`, código em `/root/lume`, subido com `docker compose`.
- Caddy: novo arquivo `/etc/caddy/sites/lume.caddy` com `reverse_proxy 127.0.0.1:3020` (o Caddy já importa `sites/*.caddy` e emite HTTPS).
- Banco: SQLite num volume do contêiner (isolado do Postgres existente).
- Nenhuma mudança na VPS sem avisar o usuário antes.

**Decisões de construção (chamadas do implementador, aprovadas junto):**
- Servidor: Node 24 + TypeScript + Hono + `node:sqlite`; testes com Vitest.
- Página de captura: Vue 3 + Vite, servida pelo mesmo servidor em `/`; API em `/api`.
- Mesma senha do cofre do desktop. O cliente deriva `segredo_de_login = Argon2id(senha, sal_da_conta)` (m=64 MiB, t=3, p=1, 32 bytes); o servidor guarda só `SHA-256(segredo_de_login)`.
- Cadastro exige um código de convite (`INVITE_CODE`, variável de ambiente), para que só vocês dois criem contas.
- Par de chaves X25519 gerado no desktop; a chave privada fica só no banco cifrado do desktop; o servidor guarda só a pública. O celular cifra com `crypto_box_seal` (libsodium). Backup da chave privada no servidor fica para o plano 4.
- A caixa de entrada guarda envelopes até o desktop confirmar o recebimento; depois apaga.
- No desktop, o plano 2 entrega: conectar a conta, receber e decifrar, ouvir áudio, ler texto, transformar ideia de texto em cena, arquivar e apagar. Transcrição e "anexar à cena" ficam para o plano 3 (IA).
