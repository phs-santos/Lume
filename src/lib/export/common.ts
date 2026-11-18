import type { JSONContent } from "@tiptap/core";

/** Onde o formato não leva a imagem (Word, PDF, Markdown): a legenda entre colchetes. */
export function figureLabel(n: JSONContent): string {
  const cap = String(n.attrs?.caption ?? "").trim();
  return cap ? `[Imagem: ${cap}]` : "[Imagem]";
}
