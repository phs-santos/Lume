import { describe, expect, it } from "vitest";
import { coverColor, COVER_COLORS } from "./coverColor";

describe("coverColor", () => {
  it("o mesmo livro tem sempre a mesma cor", () => {
    expect(coverColor("abc-123")).toBe(coverColor("abc-123"));
  });
  it("a cor vem da paleta das capas", () => {
    for (const id of ["a", "b", "c", "livro-longo-com-acentuação"]) expect(COVER_COLORS).toContain(coverColor(id));
  });
  it("livros diferentes usam cores variadas", () => {
    const ids = Array.from({ length: 30 }, (_, i) => `livro-${i}`);
    expect(new Set(ids.map(coverColor)).size).toBeGreaterThan(3);
  });
});
