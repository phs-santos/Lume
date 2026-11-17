import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

/** Marca com a classe `is-current` o bloco de topo onde está o cursor (foco + fita vermelha). */
export const CurrentBlock = Extension.create({
  name: "currentBlock",
  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("currentBlock"),
        props: {
          decorations(state) {
            const $head = state.selection.$head;
            if ($head.depth < 1) return null;
            const start = $head.before(1);
            const node = state.doc.nodeAt(start);
            if (!node) return null;
            return DecorationSet.create(state.doc, [
              Decoration.node(start, start + node.nodeSize, { class: "is-current" }),
            ]);
          },
        },
      }),
    ];
  },
});
