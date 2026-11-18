import StarterKit from "@tiptap/starter-kit";
import type { AnyExtension } from "@tiptap/core";
import { CurrentBlock } from "./currentBlock";
import { Figure } from "./figure";

/**
 * Opções do editor. Sem regras de entrada e de colagem: o que ela digita fica como digitou
 * ("- " de diálogo, "*Não*", "1984. "). Itálico e negrito só pelos atalhos Cmd/Ctrl+I e B.
 */
/** `figure`: a versão com tela (Typewriter); sem ela, o nó puro (testes, exportação). */
export function editorOptions(opts: { figure?: AnyExtension } = {}) {
  return {
    enableInputRules: false,
    enablePasteRules: false,
    extensions: [
      StarterKit.configure({
        heading: false,
        codeBlock: false,
        code: false,
        horizontalRule: false,
        bulletList: false,
        orderedList: false,
        listItem: false,
        blockquote: false,
        strike: false,
      }),
      CurrentBlock,
      opts.figure ?? Figure,
    ],
  };
}
