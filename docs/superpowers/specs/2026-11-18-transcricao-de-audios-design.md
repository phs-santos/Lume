# Transcrição dos áudios — Design (mudança média)

*Data: 2026-11-18 · Status: aguardando aprovação · Complementa §7.2 do design original e os não-objetivos do plano 3*

## Intenção e o porquê

Ela pensa falando: grava ideias no celular e hoje elas chegam ao Lume só como áudio para ouvir. O
copiar-e-colar que o Lume veio eliminar ainda existe para a voz. A transcrição fecha o ciclo: falou no
celular → o texto aparece no Lume → vira cena, entra no fim da cena ou vira conversa com o assistente.

## Objetivos

1. **Transcrição automática** de cada áudio novo que chega pela caixa de ideias, logo depois da
   sincronização, se houver chave de IA configurada. Uma por vez, em segundo plano.
2. **Literal:** o texto é o que ela falou, sem "melhorar" nada (pedido original: ela é independente).
   Idioma fixo `pt`.
3. **Nomes certos:** os nomes das fichas da Bíblia (personagens e lugares) vão como vocabulário para a
   transcrição acertar a grafia ("Teodoro", "Itaipava"). Até 100 termos.
4. **No cartão:** o player continua; embaixo aparece o texto transcrito, ou o estado ("Transcrevendo…",
   "Não deu para transcrever: …" com **Tentar de novo**, ou "Configure a chave da IA nos Ajustes").
5. **Ações** para áudio transcrito e para ideia escrita:
   - **Virar cena aqui** (já existe para texto): cena nova depois da atual, com o texto.
   - **Inserir no fim da cena atual**: pelo editor aberto, num passo de desfazer (⌘Z), igual às
     propostas da Coautora.
   - **Conversar sobre isso**: abre o Assistente com a mensagem já escrita
     ("Sobre esta ideia: «…»"), sem enviar; ela completa e envia.
6. **Gasto:** os minutos transcritos no mês aparecem nos Ajustes de IA, junto do gasto atual.

## Não-objetivos (por agora)

- Áudios maiores que 16 MB (~15 min): ficam com a mensagem "longo demais para transcrever por enquanto"
  e continuam tocáveis. O caminho assíncrono da Inworld fica para depois.
- Transcrição no próprio computador (Whisper).
- Editar o texto transcrito no cartão (edita-se depois de virar cena).
- Transcrever no celular.

## Anti-objetivos

- Reescrever, resumir ou "corrigir" o que ela falou.
- Inserir texto no livro sem clique.
- Perder o áudio porque a transcrição falhou.
- Transcrever de novo um áudio já transcrito, ou gastar sem chave configurada.
- A chave sair do Rust (mesma regra do plano 3).

## Abordagens

| Abordagem | Trade-off | Decisão |
|---|---|---|
| **A) Inworld STT** (`POST https://api.inworld.ai/stt/v1/transcribe`, modelo `inworld/inworld-stt-1`, mesma chave Basic) | Aceita MP4 e WEBM direto, tem português, uma chamada síncrona até 16 MB; o áudio passa pela Inworld (limite de confiança já aceito) e tem custo por minuto | **Escolhida pelo usuário** |
| B) Whisper local | Privado e sem custo; modelo de ~1 GB, lento em Macs antigos, sem decodificador de opus em Rust, build Windows difícil | Descartada por agora |

## Como funciona (decidido)

- **Dados:** a tabela local `inbox` ganha `transcript TEXT`, `transcript_status` (`pendente` | `pronta` |
  `erro` | `sem_chave`) e `transcript_error`. Só itens `audio` usam.
- **Chamada:** multipart (`transcribeConfig` + `file`), `audioEncoding: AUTO_DETECT`, `language: "pt"`,
  `prompts` com os nomes da Bíblia. Resposta: `transcription.transcript` e `usage.transcribedAudioMs`.
- **Quando:** depois de cada `remote_sync` que trouxe áudio, e pelo botão **Tentar de novo**. Ao configurar a
  chave, os áudios em `sem_chave` passam a `pendente` e são processados.
- **Uso:** os milissegundos transcritos somam no uso do mês com o modelo `inworld/inworld-stt-1`.
- **"Inserir no fim da cena atual"** usa o mesmo caminho das propostas (`insertAtEnd` no editor aberto).
- Áudio sem fala (transcrição vazia) fica `pronta` com texto vazio e o cartão diz "Não encontrei fala
  neste áudio".

## Deixado para quem implementa

- Mensagens exatas de erro (401 da chave, cota, rede).
- Como mostrar o texto longo no cartão (corte com "ver tudo").
