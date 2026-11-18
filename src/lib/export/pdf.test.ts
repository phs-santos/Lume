import { describe, expect, it } from "vitest";
import { book } from "./fixtures";
import { toPdf } from "./pdf";

describe("toPdf", () => {
  it("gera um PDF com folha de rosto e uma página por capítulo", () => {
    const bytes = toPdf(book);
    const head = new TextDecoder().decode(bytes.slice(0, 5));
    expect(head).toBe("%PDF-");
    const text = new TextDecoder("latin1").decode(bytes);
    expect((text.match(/\/Type \/Page\b/g) ?? []).length).toBe(3);
  });

  it("parágrafo longo quebra em várias linhas e cria páginas novas", () => {
    const long = { type: "doc", content: Array.from({ length: 40 }, () => ({ type: "paragraph", content: [{ type: "text", text: "Uma frase comprida de teste que precisa quebrar a linha várias vezes na página. ".repeat(4) }] })) };
    const bytes = toPdf({ title: "T", chapters: [{ title: "C", scenes: [{ title: "S", doc: long }] }] });
    const text = new TextDecoder("latin1").decode(bytes);
    expect((text.match(/\/Type \/Page\b/g) ?? []).length).toBeGreaterThan(3);
  });
});
