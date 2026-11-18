import type { JSONContent } from "@tiptap/core";
import { jsPDF } from "jspdf";
import type { ExportBook, ExportOptions } from "./types";

/** Página A5 de livro, em pontos. */
const W = 420, H = 595, M = 54, TOP = 64, SIZE = 11, LEAD = SIZE * 1.55, INDENT = 18;

type Word = { text: string; style: "normal" | "italic" | "bold" | "bolditalic" };

function words(nodes: JSONContent[] = []): Word[] {
  const out: Word[] = [];
  for (const n of nodes) {
    if (n.type === "hardBreak") { out.push({ text: "\n", style: "normal" }); continue; }
    const marks = new Set((n.marks ?? []).map((m) => m.type));
    const style = marks.has("bold") && marks.has("italic") ? "bolditalic" : marks.has("bold") ? "bold" : marks.has("italic") ? "italic" : "normal";
    // mantém o espaço grudado na palavra anterior para medir certo
    for (const t of (n.text ?? "").split(/(?<=\s)/)) if (t) out.push({ text: t, style });
  }
  return out;
}

function paragraphsOf(doc: JSONContent): JSONContent[][] {
  const out: JSONContent[][] = [];
  const walk = (n: JSONContent) => (n.type === "paragraph" ? out.push(n.content ?? []) : (n.content ?? []).forEach(walk));
  (doc.content ?? []).forEach(walk);
  return out;
}

export function toPdf(b: ExportBook, opts: ExportOptions = {}): Uint8Array {
  const pdf = new jsPDF({ unit: "pt", format: [W, H] });
  let y = TOP;
  let page = 1;
  const newPage = () => {
    pdf.addPage([W, H]);
    page++;
    y = TOP;
    pdf.setFont("times", "normal").setFontSize(9).text(String(page), W / 2, H - 28, { align: "center" });
  };
  const need = (h: number) => y + h > H - M && newPage();

  // folha de rosto
  pdf.setFont("times", "normal").setFontSize(22).text(pdf.splitTextToSize(b.title, W - 2 * M), W / 2, H / 3, { align: "center" });

  const heading = (t: string, size: number, gapAfter: number) => {
    pdf.setFont("times", "bold").setFontSize(size);
    for (const line of pdf.splitTextToSize(t, W - 2 * M)) {
      need(size * 1.4);
      pdf.text(line, W / 2, y, { align: "center" });
      y += size * 1.4;
    }
    y += gapAfter;
  };

  const paragraph = (nodes: JSONContent[], first: boolean) => {
    const ws = words(nodes);
    if (!ws.length) return;
    let line: Word[] = [];
    let width = first ? 0 : INDENT;
    const max = W - 2 * M;
    const flush = (indent: number) => {
      need(LEAD);
      let x = M + indent;
      for (const w of line) {
        pdf.setFont("times", w.style).setFontSize(SIZE);
        pdf.text(w.text, x, y);
        x += pdf.getTextWidth(w.text);
      }
      y += LEAD;
    };
    let indent = width;
    for (const w of ws) {
      if (w.text === "\n") { flush(indent); line = []; width = indent = 0; continue; }
      pdf.setFont("times", w.style).setFontSize(SIZE);
      const ww = pdf.getTextWidth(w.text.trimEnd());
      if (line.length && width + ww > max) {
        flush(indent);
        line = [];
        width = indent = 0;
      }
      line.push(w);
      width += pdf.getTextWidth(w.text);
    }
    if (line.length) flush(indent);
  };

  for (const c of b.chapters) {
    newPage();
    y = TOP + 40;
    heading(c.title, 16, 18);
    c.scenes.forEach((s, i) => {
      if (i > 0) {
        need(LEAD * 2);
        pdf.setFont("times", "normal").setFontSize(SIZE).text("* * *", W / 2, y + LEAD * 0.4, { align: "center" });
        y += LEAD * 1.6;
      }
      if (opts.sceneTitles && s.title.trim() && s.title.trim() !== c.title.trim()) heading(s.title, 12, 6);
      paragraphsOf(s.doc).forEach((p, k) => paragraph(p, k === 0));
    });
  }
  return new Uint8Array(pdf.output("arraybuffer"));
}
