// @vitest-environment happy-dom
import { Editor } from "@tiptap/core";
import { describe, expect, it } from "vitest";
import { editorOptions } from "./editorConfig";
import { insertFigure } from "./figure";

const make = () =>
  new Editor({
    ...editorOptions(),
    element: document.createElement("div"),
    content: { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "Antes depois" }] }] },
  });

describe("imagem no texto", () => {
  it("inserida no meio de um parágrafo divide o parágrafo e não perde texto", () => {
    const e = make();
    e.commands.setTextSelection(7); // entre "Antes " e "depois"
    insertFigure(e, "abc-123", 800, 600);
    const types = e.getJSON().content!.map((n) => n.type);
    expect(types).toEqual(["paragraph", "figure", "paragraph"]);
    expect(e.getText().replace(/\s+/g, " ").trim()).toBe("Antes depois");
  });

  it("guarda só a referência lume-img no documento, com legenda", () => {
    const e = make();
    e.commands.setTextSelection(13);
    insertFigure(e, "abc-123", 800, 600);
    const fig = e.getJSON().content!.find((n) => n.type === "figure")!;
    expect(fig.attrs).toMatchObject({ src: "lume-img:abc-123", caption: "", width: 800, height: 600 });
  });

  it("figura vinda de um documento salvo é lida de volta", () => {
    const e = new Editor({
      ...editorOptions(),
      element: document.createElement("div"),
      content: { type: "doc", content: [{ type: "figure", attrs: { src: "lume-img:x", caption: "Mapa da serra" } }] },
    });
    expect(e.getJSON().content![0]).toMatchObject({ type: "figure", attrs: { src: "lume-img:x", caption: "Mapa da serra" } });
  });
});
