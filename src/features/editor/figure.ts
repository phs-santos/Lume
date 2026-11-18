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
          const num = (k: string) => (e.getAttribute(k) ? Number(e.getAttribute(k)) : null);
          return { src: e.getAttribute("data-src"), caption: e.querySelector("figcaption")?.textContent ?? "", width: num("data-width"), height: num("data-height") };
        },
      },
    ];
  },
  renderHTML({ HTMLAttributes, node }) {
    return ["figure", mergeAttributes({ "data-lume": "", "data-src": node.attrs.src, "data-width": node.attrs.width, "data-height": node.attrs.height }, HTMLAttributes), ["figcaption", {}, node.attrs.caption ?? ""]];
  },
});

export const imageId = (src: string | null | undefined) => (src?.startsWith("lume-img:") ? src.slice(9) : null);

/**
 * Insere a imagem onde está o cursor (no meio de um parágrafo, ele se divide). Com algo
 * selecionado (ex.: outra figura), entra depois da seleção em vez de substituí-la.
 */
export function insertFigure(e: Editor, id: string, width: number, height: number) {
  const at = e.state.selection.empty ? e.state.selection.from : e.state.selection.to;
  e.chain().focus().insertContentAt(at, { type: "figure", attrs: { src: `lume-img:${id}`, caption: "", width, height } }).run();
}
