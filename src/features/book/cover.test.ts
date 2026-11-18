import { describe, expect, it } from "vitest";
import { wrapTitle } from "./cover";

describe("wrapTitle", () => {
  it("título curto fica numa linha", () => expect(wrapTitle("O Farol", 14)).toEqual(["O Farol"]));
  it("quebra em linhas equilibradas sem cortar palavra", () => {
    const lines = wrapTitle("Até ter certeza do que eu sinto", 14);
    expect(lines.join(" ")).toBe("Até ter certeza do que eu sinto");
    expect(lines.every((l) => l.length <= 14)).toBe(true);
    const lens = lines.map((l) => l.length);
    expect(Math.max(...lens) - Math.min(...lens)).toBeLessThanOrEqual(6);
  });
  it("palavra maior que a linha fica sozinha, inteira", () => {
    expect(wrapTitle("Inconstitucionalissimamente bom", 10)).toEqual(["Inconstitucionalissimamente", "bom"]);
  });
  it("espaços a mais não viram linhas vazias", () => expect(wrapTitle("  Verão   em  Paraty ", 30)).toEqual(["Verão em Paraty"]));
});
