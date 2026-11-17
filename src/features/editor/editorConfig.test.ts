// @vitest-environment happy-dom
import { Editor } from "@tiptap/core";
import { describe, expect, it } from "vitest";
import { editorOptions } from "./editorConfig";

/** Digita caractere a caractere passando pelas regras de entrada, como um teclado. */
function type(editor: Editor, text: string) {
  for (const ch of text) {
    const { from, to } = editor.state.selection;
    const handled = editor.view.someProp("handleTextInput", (f) => f(editor.view, from, to, ch, () => editor.state.tr.insertText(ch, from, to)));
    if (!handled) editor.view.dispatch(editor.state.tr.insertText(ch, from, to));
  }
}

describe("editor não transforma a prosa", () => {
  const make = () => new Editor({ ...editorOptions(), element: document.createElement("div") });

  it("*Não* continua com asteriscos, sem itálico", () => {
    const e = make();
    type(e, "Ela disse *Não* e saiu");
    expect(e.getText()).toBe("Ela disse *Não* e saiu");
    expect(JSON.stringify(e.getJSON())).not.toContain("italic");
  });

  it("- no começo do parágrafo continua hífen (diálogo), não vira lista", () => {
    const e = make();
    type(e, "- Oi, tudo bem?");
    expect(e.getText()).toBe("- Oi, tudo bem?");
    expect(JSON.stringify(e.getJSON())).not.toContain("bulletList");
  });

  it("1984. no começo não vira lista numerada", () => {
    const e = make();
    type(e, "1984. Foi o ano.");
    expect(JSON.stringify(e.getJSON())).not.toContain("orderedList");
  });
});
