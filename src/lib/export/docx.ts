import type { JSONContent } from "@tiptap/core";
import { AlignmentType, Document, HeadingLevel, Packer, Paragraph, TextRun } from "docx";
import type { ExportBook, ExportOptions } from "./types";

function runs(nodes: JSONContent[] = []): TextRun[] {
  return nodes.map((n) => {
    if (n.type === "hardBreak") return new TextRun({ text: "", break: 1 });
    const marks = new Set((n.marks ?? []).map((m) => m.type));
    return new TextRun({ text: n.text ?? "", italics: marks.has("italic"), bold: marks.has("bold") });
  });
}

/** Imagens ainda não vão para o Word: no lugar, a legenda entre colchetes. */
export function figureLabel(n: JSONContent): string {
  const cap = String(n.attrs?.caption ?? "").trim();
  return cap ? `[Imagem: ${cap}]` : "[Imagem]";
}

/** Achata blocos (citação, listas) em parágrafos de texto corrido. */
function paragraphs(doc: JSONContent): Paragraph[] {
  const out: Paragraph[] = [];
  const walk = (n: JSONContent) => {
    if (n.type === "paragraph") {
      out.push(new Paragraph({ children: runs(n.content), indent: { firstLine: 360 }, spacing: { line: 360 } }));
    } else if (n.type === "figure") {
      out.push(new Paragraph({ alignment: AlignmentType.CENTER, children: [new TextRun({ text: figureLabel(n), italics: true })] }));
    } else {
      (n.content ?? []).forEach(walk);
    }
  };
  (doc.content ?? []).forEach(walk);
  return out;
}

const separator = () => new Paragraph({ alignment: AlignmentType.CENTER, spacing: { before: 240, after: 240 }, children: [new TextRun("* * *")] });

export async function toDocx(b: ExportBook, opts: ExportOptions = {}): Promise<Uint8Array> {
  const children: Paragraph[] = [new Paragraph({ heading: HeadingLevel.TITLE, children: [new TextRun(b.title)] })];
  b.chapters.forEach((c) => {
    children.push(new Paragraph({ heading: HeadingLevel.HEADING_1, pageBreakBefore: true, children: [new TextRun(c.title)] }));
    c.scenes.forEach((s, i) => {
      if (i > 0) children.push(separator());
      if (opts.sceneTitles && s.title.trim() && s.title.trim() !== c.title.trim())
        children.push(new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun(s.title)] }));
      children.push(...paragraphs(s.doc));
    });
  });
  const doc = new Document({
    creator: "Lume",
    title: b.title,
    styles: { default: { document: { run: { font: "Georgia", size: 24 } } } },
    sections: [{ children }],
  });
  return new Uint8Array(await Packer.toArrayBuffer(doc));
}
