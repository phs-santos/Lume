import { describe, expect, it } from "vitest";
import { book } from "./fixtures";
import { toMarkdown } from "./markdown";

describe("toMarkdown", () => {
  const md = toMarkdown(book);

  it("escapa *, # e _ literais e mantém travessão e acentos", () => {
    expect(md).toContain("— Você viu? \\*Não\\*, disse ela. \\#1 e 2\\_3");
  });
  it("itálico e negrito", () => {
    expect(md).toContain("Ela riu *baixinho* e **saiu**.");
  });
  it("cenas separadas por * * *", () => {
    expect(md).toContain("**saiu**.\n\n* * *\n\nDepois.");
  });
  it("título do livro e capítulos; capítulo vazio só com o título", () => {
    expect(md.startsWith("# A Carta\n\n## Capítulo 1\n\n")).toBe(true);
    expect(md.trimEnd().endsWith("## Capítulo vazio")).toBe(true);
  });
  it("não exporta o título da cena", () => {
    expect(md).not.toContain("Cena 1");
  });
});
