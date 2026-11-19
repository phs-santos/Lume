# Criação de livro com IA e mapa de ideias — desenho

Data: 2026-11-19

## Por quê

Hoje, criar um livro no Lume é um formulário: título obrigatório e o modo da IA. Quem chega só com uma ideia vaga não tem por onde começar, e não há lugar para organizar ideias além do texto e da ficha (Bíblia). O objetivo é **melhorar a criação do livro**: começar sem título definido, desenvolver a ideia conversando com a IA e ter, durante o livro todo, um mapa de cartões ligados por setas para pensar a história.

## O que muda

### 1. Tela "Novo livro"

```
 Novo livro                                         ✕
 Nome do livro
 [ O livro da faroleira                         ]
 Pode ser um nome provisório. Dá para trocar depois.

 Como você quer começar?
 (•) Escrever direto
 ( ) Tenho uma ideia e quero desenvolver com a IA
     [ Conte a ideia do jeito que vier...           ]
     [                                              ]

 Como a IA vai ajudar neste livro
 (•) Conselheira   ( ) Coautora

 [              Criar livro               ]
```

- **Nome provisório.** O nome deixa de ser obrigatório. Vazio, o livro nasce como "Livro sem nome". O livro guarda se o nome é provisório (`books.title_provisional`): fica ligado quando o campo veio vazio e desliga quando a pessoa renomeia o livro. A capa na biblioteca mostra "nome provisório" discretamente enquanto o nome for provisório.
- **"Tenho uma ideia…"** abre um campo de texto (obrigatório nessa opção). Ao criar, o livro abre no editor já na aba **Mapa**, com o assistente aberto numa conversa nova chamada "Desenvolvendo a ideia", cuja primeira mensagem é o texto da pessoa. A IA responde em seguida.
- Sem chave de IA configurada, a opção fica desligada com a dica "Para usar, configure a IA em Ajustes do assistente". "Escrever direto" segue como hoje.

### 2. Mapa de ideias (o livro todo)

Uma aba no editor: **Texto | Mapa**. O Mapa ocupa o lugar da folha; o sumário à esquerda e o assistente à direita continuam. Cada livro tem um mapa.

```
 ┌──────────────┐        ┌──────────────┐
 │ Marina       │───────▶│ A carta some │
 │ faroleira    │        │ no cap. 3    │
 └──────────────┘        └──────┬───────┘
                                │ por quê?
                         ┌──────▼───────┐
                         │ O irmão      │
                         │ escondeu     │
                         └──────────────┘
```

- **Cartões soltos:** texto livre (várias linhas), uma de 5 cores do tema. Criar: duplo clique no vazio ou botão "+ Cartão". Editar: duplo clique no cartão. Apagar: tecla Delete/Backspace com o cartão selecionado (não durante a edição do texto) ou menu do cartão.
- **Setas:** arrastar da alça do cartão até outro cartão. A seta pode ter um rótulo curto (duplo clique na seta). Selecionar + Delete apaga.
- **Navegar:** arrastar o fundo move, roda do mouse/pinça dá zoom, botão "Ver tudo" enquadra o mapa. Seleção de vários cartões com Shift.
- **Virar ficha:** no menu do cartão, "Transformar em personagem / lugar / fato" cria a ficha na Bíblia com o texto do cartão (1ª linha = nome, resto = descrição). O cartão continua no mapa e ganha um selo com o tipo.
- Tudo salva sozinho (posição com espera curta depois de soltar o cartão), no cofre criptografado, como o resto.
- Mapa vazio mostra uma dica curta: "Dê dois cliques para criar um cartão. Puxe a bolinha de um cartão até outro para ligar."

### 3. A IA no mapa

A IA de qualquer conversa do livro (Conselheira ou Coautora) pode **ler o mapa** e **propor cartões**. Ela nunca altera o mapa sozinha.

- **Proposta de cartões:** aparece na conversa como um bloco "A IA sugere 4 cartões para o mapa", com a lista dos cartões (e das setas entre eles). Ações: **Pôr no mapa** (todos), desmarcar alguns antes, ou **Descartar**. Cartões aceitos entram juntos, arrumados numa área livre à direita do que já existe e ligados como proposto; setas para cartões que já existiam também valem. O mapa enquadra os novos.
- **Proposta de nome:** em conversas de ideia, quando a história tiver forma, a IA pode sugerir um nome; aparece como "Usar *O Farol* como nome do livro?" com **Usar** / **Agora não**. Usar renomeia o livro e desliga o "provisório" (aceitar a sugestão conta como escolha).
- **Fichas:** a IA continua usando `salvar_ficha` como hoje para personagens e lugares que firmarem na conversa.
- **Conversa de ideia:** a conversa criada pela opção "Tenho uma ideia" é marcada (`ai_conversations.purpose = 'ideia'`). Nela, as instruções da IA pedem: entender a ideia, fazer uma pergunta por vez (premissa, protagonista, o que ela quer, o que impede, mundo, tom, final possível), devolver o que entendeu e, a cada ponto que firmar, propor cartões ligados. Sem escrever o livro, em nenhum dos modos.
- A regra da Conselheira ("nunca escreve no seu texto") continua valendo: cartões não são o texto e só entram com aprovação.

## Objetivos, não-objetivos, anti-objetivos

- **Objetivos:** começar um livro sem título; desenvolver uma ideia vaga com a IA até ela virar um mapa com personagens, conflito e rumo; usar o mapa durante o livro todo.
- **Não-objetivos (agora):** desenho à mão livre; cartões ligados a cenas; exportar o mapa (imagem/PDF); mais de um mapa por livro; mapa no celular.
- **Anti-objetivos:** a IA mexer no mapa ou no texto sem aprovação; o editor ficar mais lento ou pesado ao escrever (o mapa só carrega quando a aba é aberta); qualquer dado do mapa sair do cofre; a chave da IA passar pela interface.

## Restrições

- Tauri 2 + Vue 3; dados no SQLCipher (migração nova, a 6ª); comandos Rust como os existentes; testes Vitest e Rust para o que for lógica.
- Visual nos tokens de tema do Lume (claro/escuro), não no visual padrão da biblioteca.
- O contexto da IA hoje chama o sumário de "Mapa do livro". Renomear para "Sumário do livro" (inclusive na descrição de `ler_cena`) para não confundir com o mapa de ideias.

## Abordagens consideradas

- **A. Biblioteca pronta de mapas (Vue Flow, `@vue-flow/core`, MIT).** Arrastar, setas, zoom, seleção já resolvidos; cerca de 100 KB a mais no app; visual precisa ser ajustado. **Escolhida** pela pessoa: o valor está nos cartões e na IA, não no mecanismo de arrastar.
- **B. Mapa feito do zero (SVG/HTML).** Controle total e nada a mais no app, mas bem mais trabalho e mais risco de defeitos (setas desalinhadas, zoom).
- Descartado: listas ou colunas sem área livre, porque o pedido é cartões ligados por setas.

## Dados (proposta)

```sql
ALTER TABLE books ADD COLUMN title_provisional INTEGER NOT NULL DEFAULT 0;
ALTER TABLE ai_conversations ADD COLUMN purpose TEXT NOT NULL DEFAULT 'livro'
  CHECK(purpose IN('livro','ideia'));
CREATE TABLE map_cards(
  id TEXT PRIMARY KEY, book_id TEXT NOT NULL REFERENCES books(id),
  text TEXT NOT NULL, color TEXT NOT NULL DEFAULT 'papel',
  x REAL NOT NULL, y REAL NOT NULL,
  bible_entry_id TEXT REFERENCES bible_entries(id) ON DELETE SET NULL,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
CREATE TABLE map_links(
  id TEXT PRIMARY KEY, book_id TEXT NOT NULL REFERENCES books(id),
  from_card TEXT NOT NULL REFERENCES map_cards(id) ON DELETE CASCADE,
  to_card TEXT NOT NULL REFERENCES map_cards(id) ON DELETE CASCADE,
  label TEXT NOT NULL DEFAULT '');
CREATE TABLE map_proposals(
  id TEXT PRIMARY KEY, book_id TEXT NOT NULL REFERENCES books(id),
  conversation_id TEXT REFERENCES ai_conversations(id) ON DELETE SET NULL,
  kind TEXT NOT NULL CHECK(kind IN('cartoes','nome')),
  payload_json TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN('pendente','aceita','descartada')),
  created_at INTEGER NOT NULL);
```

Ferramentas novas da IA: `ler_mapa` (cartões com id curto e setas), `propor_cartoes` (`cartoes: [{ref, texto}]`, `setas: [{de, para, rotulo?}]`, onde `de`/`para` é um `ref` novo ou id de cartão existente), `propor_nome` (só em conversa de ideia). Propostas pendentes voltam ao abrir a conversa, como as da Coautora.

## Deixado para quem constrói

- Algoritmo exato de onde pôr os cartões aceitos (grade simples à direita do conteúdo, ou em leque a partir do cartão ligado).
- Detalhes visuais do cartão, das alças e das setas dentro do tema.
- Atalho de teclado para alternar Texto/Mapa (seguir o padrão de `shortcuts.ts`) e sua entrada na Ajuda.
- Texto exato das instruções da conversa de ideia, respeitando "uma pergunta por vez" e "sem escrever o livro".

## Decisões minhas (para conferir)

- Nome vazio vira "Livro sem nome" com marca de provisório; aceitar o nome sugerido pela IA desliga a marca.
- "Tenho uma ideia" exige chave de IA e abre o editor já no Mapa com o assistente aberto.
- Cartões com 5 cores, sem tamanho ajustável; texto simples (sem negrito etc.).
- O mapa vive numa aba Texto | Mapa do editor, não numa tela separada.
- A Conselheira também propõe cartões (com aprovação).
