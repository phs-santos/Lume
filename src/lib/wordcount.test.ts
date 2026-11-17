import { describe, expect, it } from "vitest";
import { countWords } from "./wordcount";

describe("countWords", () => {
  it("travessão de diálogo não conta como palavra", () => {
    expect(countWords("— Não sei, disse ela.")).toBe(4);
  });
  it("palavra com hífen conta uma vez", () => {
    expect(countWords("guarda-chuva")).toBe(1);
  });
  it("vazio é zero", () => {
    expect(countWords("")).toBe(0);
    expect(countWords("   \n ")).toBe(0);
  });
  it("acentos e quebras de linha", () => {
    expect(countWords("Ação\nçã  é\tpor quê?")).toBe(5);
  });
  it("números contam", () => {
    expect(countWords("Em 1984 ela tinha 3 gatos")).toBe(6);
  });
});
