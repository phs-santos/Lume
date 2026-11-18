import { mergeAttributes, Node, type Editor } from "@tiptap/core";

/**
 * Imagem no meio do texto, com legenda. O documento guarda só a referência `lume-img:<id>`;
 * os bytes ficam cifrados no banco. A tela (imagem + legenda editável) é ligada no Typewriter.
 */
export const Figure = Node.create({
  name: "figure",
  group: "block",
  atom: true,
  draggable: true,
  selectable: true,
  addAttributes() {
    return {
      src: { default: null },
      caption: { default: "" },
      width: { default: null },
      height: { default: null },
    };
  },
  parseHTML() {
    return [
      {
        tag: "figure[data-lume]",
        getAttrs: (el) => {
          const e = el as HTMLElement;
          return { src: e.getAttribute("data-src"), caption: e.querySelector("figcaption")?.textContent ?? "" };
        },
      },
    ];
  },
  renderHTML({ HTMLAttributes, node }) {
    return ["figure", mergeAttributes({ "data-lume": "", "data-src": node.attrs.src }, HTMLAttributes), ["figcaption", {}, node.attrs.caption ?? ""]];
  },
});

export const imageId = (src: string | null | undefined) => (src?.startsWith("lume-img:") ? src.slice(9) : null);

/** Insere a imagem onde está o cursor (no meio de um parágrafo, ele se divide). */
export function insertFigure(e: Editor, id: string, width: number, height: number) {
  e.chain().focus().insertContent({ type: "figure", attrs: { src: `lume-img:${id}`, caption: "", width, height } }).run();
}
