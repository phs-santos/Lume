# Plano 3 — Assistente de IA e Bíblia viva — Design

*Data: 2026-11-17 · Status: aguardando aprovação · Complementa `2026-11-17-maquina-de-escrever-design.md` §7.3–7.4*

## 1. Intenção e o porquê

Um painel de conversa com IA à direita do editor. Ele precisa conhecer o livro, **não perder memória**
(nem da conversa, nem do que já se sabe sobre a história) e **não se perder**: não confunde personagens,
não contradiz fatos já escritos e sabe em que cena a pessoa está.

O autor pediu explicitamente que os agentes sejam **eficientes e econômicos**, estruturados de forma
prática. A **Bíblia viva**, que o design original deixava para a v2, entra agora.

## 2. Objetivos

1. **Painel "Assistente"** à direita da folha, com abas **Conversa** e **Bíblia**. Abre por um ícone na
   barra de cima e por um atalho. Fica ao lado da folha (a folha estreita, nada é coberto) e continua
   aberto ao trocar de cena.
   O processamento de fundo só roda em livros onde o Assistente já foi aberto ao menos uma vez e com
   chave configurada *(decisão do construtor-designer: custo só onde há uso)*.
2. **Conversa contínua por livro**, salva no banco local cifrado. Respostas em streaming. Ela sabe sempre
   a cena aberta. Quando consulta o livro, mostra uma linha discreta ("consultei a cena X"). O botão
   **Nova conversa** começa outra. As anteriores ficam numa lista simples ("Conversas anteriores") e podem
   ser reabertas. O resumo delas **não** vai no contexto fixo; a IA o consulta pela ferramenta
   `ler_conversas_anteriores` quando precisa *(decisão: contexto não cresce a cada conversa)*.
3. **Bíblia viva**: fichas de **personagens, lugares, fatos e linha do tempo**. Cada ficha guarda a cena
   de origem. A IA cria e atualiza as fichas, e a pessoa pode editar ou apagar qualquer uma.
4. **Atualização em segundo plano**: uma cena é reprocessada (resumo curto + fichas) quando a pessoa sai
   dela ou fica **2 minutos sem digitar**, e **só se o texto mudou** desde a última vez (hash do conteúdo).
   Num livro importado, a primeira passada por todas as cenas acontece ao abrir o Assistente pela
   primeira vez naquele livro, com barra de progresso, em segundo plano e cancelável.
5. **Modos por livro** (campo `ai_mode` já existente):
   - **Conselheira**: conversa e mexe na Bíblia; **nunca** no manuscrito. Isso é garantido no backend:
     as ferramentas de escrita no texto não existem nesse modo.
   - **Coautora**: além disso, pode **propor trechos**. Eles aparecem como cartão com
     **Inserir no fim da cena** / **Virar cena nova** / **Descartar**. Nada entra sem clique.
6. **Ajustes de IA**: chave da Inworld, modelo da conversa, modelo de fundo e um **contador de gasto do
   mês** (tokens e custo estimado, quando o provedor informar).

## 3. Não-objetivos (por agora)

- Transcrever os áudios da caixa de ideias e "anexar à cena" (próximo candidato).
- "Conversar com a IA sobre esta ideia" nos cartões da caixa de entrada (vem junto com a transcrição).
- Propor edições no meio do texto (diff). A Coautora só propõe trechos novos.
- Busca por vetores/embeddings (RAG).
- OpenRouter: a interface deixa a porta aberta, mas só a Inworld é implementada e testada agora.
- Sincronizar conversa ou Bíblia entre computadores.

## 4. Anti-objetivos

- A IA alterar uma palavra do texto no modo Conselheira, ou na Coautora sem aprovação.
- A chave de API ir para o código, para o repositório ou para a camada web (webview). Ela só vive no
  banco cifrado e só é lida pelo Rust.
- Mandar o livro inteiro em cada mensagem (custo que cresce com o livro).
- Perder a conversa ao fechar o app, ou perder a Bíblia por erro de uma chamada de IA.
- Processar em segundo plano a cada tecla, ou reprocessar uma cena que não mudou.
- O painel atrapalhar a escrita: nada de pop-ups; tudo cabe no painel.

## 5. Restrições

- App: Tauri 2 (Rust) + Vue 3/TS. Banco SQLCipher com migrações em `src-tauri/src/db/mod.rs`. HTTP com
  `reqwest` (blocking) dentro de `spawn_blocking`, como já fazem `remote_sync` e o import de EPUB.
- Provedor: **Inworld Router**, compatível com OpenAI. `POST https://api.inworld.ai/v1/chat/completions`,
  header `Authorization: Basic <chave>`, `stream: true` (SSE), `tools` no formato OpenAI. A lista de
  modelos fica em `GET https://api.inworld.ai/llm/v1alpha/models`. Chave testada em 2026-11-17: funciona.
- Modelos padrão (trocáveis em Ajustes): conversa `anthropic/claude-sonnet-5-5`; fundo
  `anthropic/claude-haiku-5-5`.
- O texto do livro vai para a Inworld e para o fornecedor do modelo. Isso já é um limite de confiança
  aceito no design original (§6.4).
- Interface em português do Brasil, no visual atual (barra escura, ícones com dica).

## 6. Abordagens

| Abordagem | Trade-off | Decisão |
|---|---|---|
| A) Mandar o livro inteiro a cada mensagem | Simples, mas caro e lento, e piora com o tamanho do livro | Descartada (anti-objetivo de custo) |
| **B) Bíblia viva + resumos + ferramentas sob demanda, com dois modelos** | Contexto pequeno e estável; a IA consulta em vez de adivinhar; exige o processamento em segundo plano | **Escolhida** |
| C) RAG com embeddings | Bom para acervos enormes; estrutura demais para livros deste tamanho | Descartada por agora |

## 7. Como funciona (decidido)

### 7.1 O que vai em cada pergunta (contexto montado pelo Rust)

1. **Instruções fixas**: o papel conforme o modo, as regras e o idioma.
2. **Bíblia compacta**: os nomes e uma linha de cada ficha.
3. **Mapa do livro**: capítulos e cenas com o resumo curto de cada cena.
4. **Cena atual**: o texto inteiro.
5. **Memória da conversa**: o resumo acumulado das mensagens antigas, mais as **últimas ~12 mensagens**
   na íntegra.

A ordem é do mais estável para o mais variável, para aproveitar o cache de prompt do provedor quando ele
existir. Quando as mensagens fora da janela passam de um limite, o modelo de fundo **dobra** essas
mensagens no resumo acumulado. A conversa inteira continua salva; só deixa de ir na íntegra.

### 7.2 Ferramentas do agente da conversa

- `buscar_no_livro(texto)`: busca por palavras nas cenas e devolve trechos com cena e capítulo (busca
  simples no SQLite, sem vetores).
- `ler_cena(id)`: texto completo de uma cena.
- `ler_ficha(id ou nome)`: ficha completa da Bíblia.
- `salvar_ficha(id?, tipo, nome, apelidos?, fato, cena_origem?)`: com `id`, acrescenta à ficha; sem `id`,
  procura antes por (livro, tipo, nome ou apelido), sem diferenciar maiúsculas e acentos, e só cria se
  não achar. Segue a mesma regra de fichas editadas à mão (§7.3).
- `ler_conversas_anteriores()`: título e resumo das conversas encerradas do livro.
- **Só na Coautora:** `propor_trecho(texto)`, que cria o cartão de proposta e não escreve nada.

Há um limite de **6 chamadas de ferramenta por resposta**, para não entrar em laço e gastar à toa.

### 7.3 Agente de fundo (por cena)

**Identidade das fichas:** cada ficha tem `id` estável, `nome` e uma lista de **apelidos** ("Rafa",
"Rafael"). A Bíblia compacta mostra `id`, nome e apelidos, para a IA atualizar a ficha certa em vez de
criar uma duplicada. A pessoa pode renomear e editar apelidos; o `id` não muda.

**Conteúdo das fichas:** dois campos. `corpo` é o texto principal; `notas_ia` é uma lista de fatos curtos
acrescentados pela IA, cada um com a cena de origem. Enquanto a ficha não foi editada à mão, a IA pode
reescrever o `corpo`. Depois que a pessoa edita, a IA (de fundo **ou** da conversa) só acrescenta em
`notas_ia`, e a pessoa decide se incorpora. A tela mostra os dois.

**Linha do tempo:** fichas desse tipo têm um campo `ordem` (inteiro) e aparecem ordenadas na ordem da
história. A IA escolhe a posição relativa; a pessoa pode arrastar para corrigir.

Entrada: o texto da cena + a Bíblia compacta. Saída em JSON: `resumo` (2–3 frases) e uma lista de
alterações de fichas (criar/atualizar), cada uma com a cena de origem. Ele não apaga ficha; quem apaga
é a pessoa. Fichas editadas à mão ficam marcadas como **"editada por você"** e a IA não reescreve o
conteúdo delas; pode só acrescentar um fato novo. Falha de chamada não muda nada: a cena fica pendente
e é tentada de novo depois.

### 7.4 Dados novos (migração)

- `ai_scene_digest(scene_id, content_hash, summary, updated_at)`
- `bible_entries(id, book_id, kind ∈ {personagem, lugar, fato, linha_do_tempo}, name, aliases_json, body, ai_notes_json, story_order, source_scene_id, edited_by_user, updated_at)`
- `ai_conversations(id, book_id, title, rolling_summary, closed, created_at, updated_at)`
- `ai_messages(id, conversation_id, role, content, tool_name, created_at)`
- `ai_proposals(id, book_id, conversation_id, scene_id, text, status ∈ {pendente, inserida, virou_cena, descartada}, created_at)`
- `ai_usage(month, model, input_tokens, output_tokens, cost)`
- Chave e modelos em `settings` (`ai.inworld_key`, `ai.chat_model`, `ai.background_model`).

Apagar uma cena não apaga as fichas que vieram dela; a origem fica nula.

### 7.5 Interface

```
┌ sumário ┐┌──────── folha ────────┐┌─ Assistente ── Conversa | Bíblia ┐
│         ││  Linha 340            ││ você: o Rafa já sabia do bug?   │
│         ││  A tela do notebook…  ││ IA: Não. Ele só descobre na     │
│         ││                       ││  cena "A Porta do Elevador"     │
│         ││                       ││  · consultei a cena             │
│         ││                       ││ [ escreva…              ] (➤)   │
└─────────┘└───────────────────────┘└─────────────────────────────────┘
```

- Sem chave configurada, o painel mostra só um campo para colar a chave, com um link para os Ajustes.
- Erros de rede ou de cota aparecem como uma linha no próprio painel, com **Tentar de novo**.
- **Propostas da Coautora:** "Inserir no fim da cena" passa **pelo editor aberto**: se a cena da proposta
  não está aberta, o app a abre primeiro; o trecho entra como parágrafos no fim, num único passo de
  desfazer (⌘Z), e o autosave normal grava. Nunca se escreve direto no banco com a cena aberta, para o
  autosave não sobrescrever. "Virar cena nova" cria uma cena logo depois da cena da proposta, no mesmo
  capítulo, com o trecho como conteúdo, e a abre.
- A aba Bíblia lista as fichas agrupadas por tipo, com busca. Clicar abre a ficha para editar. A origem
  é um link que abre a cena.
- Enquanto a primeira passada roda: "Lendo o livro… 12/48 cenas".

## 8. Deixado para quem implementa

- Texto exato das instruções fixas de cada modo e dos prompts do agente de fundo.
- Limite em tokens para dobrar a conversa no resumo e tamanho exato da janela de mensagens.
- Se o cache de prompt funciona através da Inworld. Se não funcionar, a ordem do contexto continua
  certa e nada mais muda.
- Atalho de teclado do painel (sem conflitar com os existentes em `shortcuts.ts`) e largura do painel.
- Como o streaming chega ao front (Tauri `Channel` ou eventos).
- Custo estimado: usar o que a resposta informar; sem isso, mostrar só tokens.
