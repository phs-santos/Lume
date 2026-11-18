// @vitest-environment happy-dom
import { Editor } from "@tiptap/core";
import { afterEach, describe, expect, it } from "vitest";
import { editorOptions } from "./editorConfig";
import { insertFigure } from "./figure";

const editors: Editor[] = [];
afterEach(() => editors.splice(0).forEach((e) => e.destroy()));
const track = (e: Editor) => (editors.push(e), e);
const make = () =>
  track(new Editor({
    ...editorOptions(),
    element: document.createElement("div"),
    content: { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "Antes depois" }] }] },
  }));

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
    const e = track(new Editor({
      ...editorOptions(),
      element: document.createElement("div"),
      content: { type: "doc", content: [{ type: "figure", attrs: { src: "lume-img:x", caption: "Mapa da serra" } }] },
    }));
    expect(e.getJSON().content![0]).toMatchObject({ type: "figure", attrs: { src: "lume-img:x", caption: "Mapa da serra" } });
  });

  it("colar com uma figura selecionada não a substitui", () => {
    const e = make();
    e.commands.setTextSelection(13);
    insertFigure(e, "um", 10, 10);
    const pos = e.getJSON().content!.findIndex((n) => n.type === "figure");
    let at = 0;
    e.state.doc.forEach((_n, off, i) => { if (i === pos) at = off; });
    e.commands.setNodeSelection(at);
    insertFigure(e, "dois", 10, 10);
    const srcs = e.getJSON().content!.filter((n) => n.type === "figure").map((n) => n.attrs!.src);
    expect(srcs).toEqual(["lume-img:um", "lume-img:dois"]);
  });

  it("largura e altura sobrevivem a copiar e colar (HTML)", () => {
    const e = make();
    e.commands.setContent('<figure data-lume="" data-src="lume-img:x" data-width="800" data-height="600"><figcaption>Serra</figcaption></figure>');
    expect(e.getJSON().content![0].attrs).toMatchObject({ src: "lume-img:x", caption: "Serra", width: 800, height: 600 });
    expect(e.getHTML()).toContain('data-width="800"');
  });
});
