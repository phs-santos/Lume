# Editor enxuto — Design (mudança pequena)

*Data: 2026-11-17 · Status: aguardando aprovação*

## Intenção
O editor deve ser simples de usar, não confuso e sem espaços vazios. Hoje a lateral repete nomes,
a barra de cima tem botões demais com texto, e sobra folha em branco no fim da cena.

## Objetivos
1. **Lateral sem repetição.** Capítulo com uma cena só, de mesmo título, vira **uma linha só**
   (título do capítulo + contagem). Clicar abre a cena. A linha fica marcada quando é a cena atual.
   Capítulos com várias cenas continuam como hoje.
2. **"+ Cena" só no hover** do capítulo (o "Novo capítulo" no fim continua visível).
3. **Barra de cima só com ícones.** Ideias, Versões, Exportar e Ajustes perdem o texto. O nome aparece
   na dica ao passar o mouse, e o selo de ideias novas continua. "← Livros" vira só a seta.
4. **Título fora da barra.** O campo de título da cena sai da barra de cima. Para renomear uma cena,
   dê dois cliques nela na lateral, igual ao capítulo hoje.
5. **Topo da lateral sem corte.** Os títulos não podem aparecer cortados embaixo do cabeçalho
   fixo. O cabeçalho cobre por inteiro, com uma leve sombra de borda.
6. **Menos vazio no fim da cena.** A folga embaixo cai de meia tela para cerca de um terço da tela.
   Ainda dá para a última linha subir bem, sem deixar um bloco branco grande.

## Não-objetivos
Modo foco escondido, mudar a cor e o tema, capítulos que abrem e fecham, ou nova tela de organizar.

## Anti-objetivos
Esconder algo de forma que a pessoa não ache. Por isso nada some sem aparecer no hover, e todo ícone tem dica.

## Abordagens
- **A) Enxugar o que existe** — escolhida: menor risco, mantém a cara do app.
- B) Modo foco que esconde lateral e barra — descartada: o que fica escondido confunde.
- C) Refazer a navegação — descartada por agora: muito trabalho e muda a cara do app.

## Deixado para quem implementa
- Arrastar cenas de/para um capítulo de linha única deve continuar funcionando. Se não der de forma
  simples, a linha única vira a própria cena arrastável.
- Valor exato da folga e da sombra do cabeçalho: ajustar olhando.
