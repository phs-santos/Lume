import JSZip from "jszip";
import { describe, expect, it } from "vitest";
import { toDocx } from "./docx";
import { book } from "./fixtures";

describe("toDocx", () => {
  it("gera um .docx com o texto e os capítulos", async () => {
    const bytes = await toDocx(book);
    const zip = await JSZip.loadAsync(bytes);
    const xml = await zip.file("word/document.xml")!.async("string");
    expect(xml).toContain("— Você viu? *Não*, disse ela. #1 e 2_3");
    expect(xml).toContain("Capítulo 1");
    expect(xml).toContain("* * *");
  });
});
