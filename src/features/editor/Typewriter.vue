<script setup lang="ts">
import { EditorContent, useEditor } from "@tiptap/vue-3";
import { onBeforeUnmount, ref } from "vue";
import { countWords } from "../../lib/wordcount";
import { editorOptions } from "./editorConfig";
import { playKey } from "./keySound";

const props = defineProps<{ sceneId: string; contentJson: string; focusMode: boolean; keySound: boolean }>();
const emit = defineEmits<{ change: [sceneId: string, json: string, words: number] }>();
const scroller = ref<HTMLElement | null>(null);

function parse(json: string) {
  try {
    return JSON.parse(json);
  } catch {
    return "";
  }
}

/** Mantém a linha do cursor no meio da tela, como o rolo de uma máquina de escrever. */
function centerCaret() {
  const ed = editor.value;
  const box = scroller.value;
  if (!ed || !box) return;
  const caret = ed.view.coordsAtPos(ed.state.selection.head);
  const rect = box.getBoundingClientRect();
  const target = rect.top + rect.height / 2;
  box.scrollTop += caret.top - target;
}

const editor = useEditor({
  content: parse(props.contentJson),
  autofocus: "end",
  ...editorOptions(),
  editorProps: {
    attributes: { spellcheck: "true", lang: "pt-BR", "aria-label": "Texto da cena" },
    handleKeyDown: (_view, e) => {
      if (props.keySound && (e.key.length === 1 || e.key === "Enter" || e.key === "Backspace")) playKey();
      return false;
    },
  },
  onUpdate: ({ editor: ed }) => {
    emit("change", props.sceneId, JSON.stringify(ed.getJSON()), countWords(ed.getText({ blockSeparator: "\n" })));
  },
  onSelectionUpdate: () => requestAnimationFrame(centerCaret),
  onCreate: () => requestAnimationFrame(centerCaret),
});

onBeforeUnmount(() => editor.value?.destroy());
</script>

<template>
  <div ref="scroller" :class="['typewriter', { focus: focusMode }]">
    <EditorContent :editor="editor" class="page" />
  </div>
</template>

<style scoped>
.typewriter {
  height: 100%;
  overflow-y: auto;
  scrollbar-width: thin;
  background: var(--desk);
  padding: 0 1.5rem;
}
/* A folha: uma tira contínua de papel, como saindo do rolo da máquina. */
.page {
  max-width: calc(68ch + 9rem);
  min-height: 100%;
  margin: 0 auto;
  /* meia tela de folga em cima e embaixo: a primeira e a última linha também chegam ao centro */
  padding: 50vh clamp(1.5rem, 6vw, 4.5rem);
  background: var(--sheet);
  box-shadow: 0 0 0 1px rgba(28, 35, 51, 0.05), 0 20px 50px -24px rgba(28, 35, 51, 0.35);
}
.page :deep(.ProseMirror) {
  font-family: var(--font-text);
  font-size: 1.15rem;
  line-height: 1.75;
  color: var(--ink);
  caret-color: var(--ribbon);
  outline: none;
}
.page :deep(.ProseMirror p) { margin: 0; text-indent: 1.5em; }
.page :deep(.ProseMirror p:first-child) { text-indent: 0; }
.page :deep(.ProseMirror blockquote) { margin: 1em 0 1em 1.5em; font-style: italic; }

/* A fita: traço vermelho na margem, na altura do parágrafo atual. */
.page :deep(.ProseMirror > .is-current) { position: relative; }
.page :deep(.ProseMirror > .is-current)::before {
  content: "";
  position: absolute;
  left: -1.25rem;
  top: 0.35em;
  bottom: 0.35em;
  width: 2px;
  background: var(--ribbon);
}

.focus .page :deep(.ProseMirror > *) { opacity: 0.3; transition: opacity 0.2s; }
.focus .page :deep(.ProseMirror > .is-current) { opacity: 1; }
</style>
