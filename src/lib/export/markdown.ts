import type { JSONContent } from "@tiptap/core";
import type { ExportBook, ExportOptions } from "./types";

export type { ExportBook };

/** Escapa caracteres que o Markdown leria como marcação, para a prosa sair literal. */
const escape = (s: string) => s.replace(/([\\`*_#\[\]<>|])/g, "\\$1");

function inline(nodes: JSONContent[] = []): string {
  return nodes
    .map((n) => {
      if (n.type === "hardBreak") return "  \n";
      let text = escape(n.text ?? "");
      const marks = new Set((n.marks ?? []).map((m) => m.type));
      if (marks.has("italic")) text = `*${text}*`;
      if (marks.has("bold")) text = `**${text}**`;
      return text;
    })
    .join("");
}

function block(n: JSONContent): string {
  switch (n.type) {
    case "figure": {
      const cap = String(n.attrs?.caption ?? "").trim();
      return cap ? `[Imagem: ${escape(cap)}]` : "[Imagem]";
    }
    case "paragraph":
      return inline(n.content);
    case "blockquote":
      return (n.content ?? []).map(block).join("\n\n").replace(/^/gm, "> ");
    case "bulletList":
      return (n.content ?? []).map((li) => `- ${(li.content ?? []).map(block).join(" ")}`).join("\n");
    case "orderedList":
      return (n.content ?? []).map((li, i) => `${i + 1}. ${(li.content ?? []).map(block).join(" ")}`).join("\n");
    default:
      return inline(n.content);
  }
}

const docToMd = (doc: JSONContent) =>
  (doc.content ?? []).map(block).filter((b) => b.trim() !== "").join("\n\n");

export function toMarkdown(b: ExportBook, opts: ExportOptions = {}): string {
  const chapters = b.chapters.map((c) => {
    const titled = (s: { title: string }) => opts.sceneTitles && s.title.trim() && s.title.trim() !== c.title.trim();
    const scenes = c.scenes.map((s) => [titled(s) ? `### ${s.title}` : "", docToMd(s.doc)].filter(Boolean).join("\n\n")).filter(Boolean);
    return [`## ${c.title}`, scenes.join("\n\n* * *\n\n")].filter(Boolean).join("\n\n");
  });
  return `# ${b.title}\n\n${chapters.join("\n\n")}\n`;
}
