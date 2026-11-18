import { describe, expect, it } from "vitest";
import type { InboxItem } from "../../lib/types";
import { aboutIdea, ideaText, transcriptLabel } from "./ideaText";

const audio = (patch: Partial<InboxItem> = {}): InboxItem => ({
  id: "a", kind: "audio", mime: "audio/mp4", size: 1, created_at: 0, status: "nova", text: null,
  transcript: null, transcript_status: "pendente", transcript_error: null, ...patch,
});
const text = (t: string): InboxItem => ({ ...audio(), kind: "text", text: t, transcript_status: null });

describe("ideaText", () => {
  it("ideia escrita devolve o texto", () => expect(ideaText(text("E se ela souber?"))).toBe("E se ela souber?"));
  it("áudio transcrito devolve a transcrição", () =>
    expect(ideaText(audio({ transcript_status: "pronta", transcript: "Marina foge." }))).toBe("Marina foge."));
  it("áudio sem fala não tem texto", () => expect(ideaText(audio({ transcript_status: "pronta", transcript: "  " }))).toBeNull());
  it("áudio ainda pendente não tem texto", () => expect(ideaText(audio())).toBeNull());
});

describe("transcriptLabel", () => {
  it("pendente", () => expect(transcriptLabel(audio())).toBe("Transcrevendo…"));
  it("pendente depois de cair a rede mostra o aviso", () =>
    expect(transcriptLabel(audio({ transcript_error: "Sem conexão com a Inworld; tento de novo depois." }))).toBe(
      "Sem conexão com a Inworld; tento de novo depois.",
    ));
  it("sem fala", () => expect(transcriptLabel(audio({ transcript_status: "pronta", transcript: "" }))).toBe("Não encontrei fala neste áudio."));
  it("pronta com texto não tem rótulo", () => expect(transcriptLabel(audio({ transcript_status: "pronta", transcript: "x" }))).toBeNull());
  it("erro mostra o motivo", () =>
    expect(transcriptLabel(audio({ transcript_status: "erro", transcript_error: "Chave da Inworld inválida" }))).toBe(
      "Não deu para transcrever: Chave da Inworld inválida",
    ));
  it("sem chave orienta para os Ajustes", () =>
    expect(transcriptLabel(audio({ transcript_status: "sem_chave" }))).toBe("Configure a chave da IA nos Ajustes para transcrever."));
  it("ideia escrita não tem rótulo", () => expect(transcriptLabel(text("x"))).toBeNull());
});

describe("aboutIdea", () => {
  it("monta o começo da conversa com a ideia entre aspas", () =>
    expect(aboutIdea("E se ela souber?")).toBe("Sobre esta ideia: «E se ela souber?»\n\n"));
});
