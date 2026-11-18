import type { InboxItem } from "../../lib/types";

/** Texto da ideia: o que ela escreveu, ou o que falou (transcrição pronta e com fala). */
export function ideaText(i: InboxItem): string | null {
  if (i.kind === "text") return i.text?.trim() ? i.text : null;
  if (i.transcript_status === "pronta" && i.transcript?.trim()) return i.transcript;
  return null;
}

/** Linha de estado da transcrição no cartão de áudio (`null` quando o texto já aparece). */
export function transcriptLabel(i: InboxItem): string | null {
  if (i.kind !== "audio") return null;
  switch (i.transcript_status) {
    case "pronta":
      return i.transcript?.trim() ? null : "Não encontrei fala neste áudio.";
    case "erro":
      return `Não deu para transcrever: ${i.transcript_error ?? "erro desconhecido"}`;
    case "sem_chave":
      return "Configure a chave da IA nos Ajustes para transcrever.";
    default:
      // pendente: pode estar esperando a rede voltar
      return i.transcript_error ?? "Transcrevendo…";
  }
}

/** Começo da conversa com o Assistente sobre uma ideia (ela completa e envia). */
export function aboutIdea(text: string): string {
  return `Sobre esta ideia: «${text.trim()}»\n\n`;
}
