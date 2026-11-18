# Imagens no texto e EPUB caprichado — Design

*Data: 2026-11-18 · Status: aguardando aprovação*

## Intenção e o porquê

Vocês querem levar o livro para ler no Kindle (e no Apple Books, Google Play Livros, celular) com cara de
livro de verdade: capa, página de rosto, sumário que funciona, contracapa — e com imagens no meio do texto
(ilustrações, mapas, fotos). Hoje o Lume exporta Word, PDF e Markdown, mas sem capa, sem imagens e sem EPUB.

## Objetivos

1. **Imagens no editor.** Colar, arrastar ou escolher uma imagem para inseri-la no ponto do cursor, entre
   parágrafos, com uma **legenda opcional** embaixo. Apagar e desfazer (⌘Z) como qualquer texto.
2. **Guardadas cifradas** no banco do livro, como o texto. Nada vai para pasta aberta no disco.
3. **Ficha do livro** (um painel novo, "Livro" nos Ajustes ou no menu do livro): **autor**, **sinopse**,
   **dedicatória** (opcional), **capa** e **contracapa**.
4. **Capa:** uma imagem escolhida por vocês, **ou** uma capa gerada pelo Lume com título e autor, no visual
   do app (cor da capa do livro na biblioteca, tipografia do Lume). A capa gerada vira imagem dentro do EPUB,
   porque o Kindle exige capa em imagem.
5. **Exportar EPUB** com, nesta ordem: capa · página de rosto (título e autor) · dedicatória (se houver) ·
   **sumário clicável** (capítulos) · capítulos com as imagens e legendas · **contracapa** (sinopse e, se
   houver, a imagem da contracapa).
6. **Funciona no Kindle:** EPUB 3 com sumário também no formato antigo (NCX), que alguns leitores ainda usam,
   e o arquivo passa no validador oficial (`epubcheck`).

## Não-objetivos (por agora)

- Publicar em loja (ISBN, exigências da Amazon KDP, mais metadados).
- Imagem flutuando ao lado do texto, recorte ou edição da imagem dentro do Lume.
- Imagens no Word e no PDF (continuam exportando só o texto; a imagem vira a legenda entre colchetes).
- Escolher entre vários modelos de capa (uma capa gerada bem-feita primeiro).
- Sincronizar imagens pelo celular.

## Anti-objetivos

- Imagens deixarem o banco pesado demais: fotos enormes são reduzidas ao inserir.
- EPUB que abre feio ou quebrado no Kindle.
- Imagem sumir do texto ao trocar de cena, restaurar versão ou importar.
- Imagem guardada fora do cofre cifrado.

## Abordagens

| Abordagem | Trade-off | Decisão |
|---|---|---|
| **Imagens dentro do banco cifrado; EPUB montado no app (como as outras exportações)** | Privado, faz backup junto com o livro; o banco cresce com as imagens (por isso reduzimos ao inserir) | **Escolhida** |
| Imagens em arquivos numa pasta | Banco menor; mas as imagens ficariam fora do cofre, sem cifra | Descartada (privacidade) |
| EPUB por serviço externo/conversor | Menos código; mas o livro sairia do computador | Descartada |

## Como funciona (proposto)

- **Banco:** tabela nova `images(id, book_id, mime, bytes, width, height, created_at)`. A cena guarda só uma
  referência (`lume-img:<id>`) no nó de imagem do editor, com a legenda.
- **Ao inserir:** a imagem é reduzida para no máximo 1600 px no lado maior e salva em JPEG (ou PNG, se tiver
  transparência). Limite de 10 MB por imagem antes de reduzir.
- **No editor:** o nó de imagem mostra a figura centralizada, com a legenda em itálico; clicar seleciona,
  Delete apaga. Inserir por colar, arrastar ou um botão "Imagem" no menu da cena.
- **Versões e apagar:** a imagem continua no banco enquanto alguma versão de cena usar a referência; uma
  limpeza remove as que ninguém usa.
- **Capa gerada:** desenhada no app em 1600 × 2560 px (proporção recomendada pela Amazon), com título, autor
  e a cor da capa do livro.
- **EPUB:** gerado no app com a biblioteca de zip que já usamos; fonte do texto Literata embutida (se o
  tamanho couber) ou a fonte do leitor; sumário `nav.xhtml` + `toc.ncx`.

## Deixado para quem implementa

- Desenho exato da capa gerada e da página de rosto.
- Onde fica o botão "Imagem" (menu ⋯ da cena ou barra).
- Se a fonte vai embutida no EPUB (depende do tamanho final).
