import StarterKit from "@tiptap/starter-kit";
import { CurrentBlock } from "./currentBlock";

/**
 * Opções do editor. Sem regras de entrada e de colagem: o que ela digita fica como digitou
 * ("- " de diálogo, "*Não*", "1984. "). Itálico e negrito só pelos atalhos Cmd/Ctrl+I e B.
 */
export function editorOptions() {
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
    ],
  };
}
