<script setup lang="ts">
import { EditorContent, useEditor } from "@tiptap/vue-3";
import { onBeforeUnmount, ref } from "vue";
import { countWords } from "../../lib/wordcount";
import { editorOptions } from "./editorConfig";
import { playKey } from "./keySound";

const props = defineProps<{ sceneId: string; contentJson: string; focusMode: boolean; keySound: boolean; chapterTitle: string; words: number }>();
const emit = defineEmits<{ change: [sceneId: string, json: string, words: number] }>();
const scroller = ref<HTMLElement | null>(null);

function parse(json: string) {
  try {
    return JSON.parse(json);
  } catch {
    return "";
  }
}

/** Mantém a linha do cursor fixa, como o rolo de uma máquina de escrever. */
function centerCaret() {
  const ed = editor.value;
  const box = scroller.value;
  if (!ed || !box) return;
  const caret = ed.view.coordsAtPos(ed.state.selection.head);
  const rect = box.getBoundingClientRect();
  box.scrollTop += caret.bottom - (rect.top + rect.height * 0.5);
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

/** A roda do mouse sobre a mesa (fora da folha) também rola a folha. */
function forwardWheel(e: WheelEvent) {
  const box = scroller.value;
  if (box && !box.contains(e.target as Node)) box.scrollTop += e.deltaY;
}
</script>

<template>
  <div :class="['typewriter', { focus: focusMode }]" @wheel.passive="forwardWheel">
    <div ref="scroller" class="sheet">
      <article class="page">
        <header class="running-head">
          <span class="chapter">{{ chapterTitle }}</span>
          <span class="rule" aria-hidden="true" />
        </header>
        <EditorContent :editor="editor" />
        <footer class="page-foot">{{ words.toLocaleString("pt-BR") }} {{ words === 1 ? "palavra" : "palavras" }} nesta cena</footer>
      </article>
    </div>
  </div>
</template>

<style scoped>
/* A mesa (madeira) fica parada; só a folha rola. */
.typewriter {
  position: relative;
  height: 100%;
  overflow: hidden;
  display: flex;
  justify-content: center;
  padding: 0 1.5rem;
  background:
    repeating-linear-gradient(92deg, rgba(0, 0, 0, 0.05) 0 2px, transparent 2px 11px, rgba(255, 255, 255, 0.03) 11px 13px, transparent 13px 23px),
    linear-gradient(180deg, var(--wood-top), var(--wood-bottom));
}
/* A folha: uma tira contínua de papel, como saindo do rolo da máquina. */
.sheet {
  width: 100%;
  max-width: calc(68ch + 9rem);
  height: 100%;
  overflow-y: auto;
  overscroll-behavior: contain;
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--ink) 25%, transparent) transparent;
  /* papel: branco quente com fibra bem leve */
  background:
    url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='2' stitchTiles='stitch'/%3E%3CfeColorMatrix values='0 0 0 0 .45 0 0 0 0 .4 0 0 0 0 .32 0 0 0 .05 0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E"),
    var(--sheet);
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.08), 0 30px 60px -20px rgba(0, 0, 0, 0.55), inset 0 0 80px rgba(120, 100, 70, 0.06);
}
.sheet:not(:hover) { scrollbar-color: transparent transparent; }
/* meia tela de folga em cima e embaixo: a primeira e a última linha também chegam ao centro */
.page { padding: 50vh clamp(1.5rem, 7vw, 5.5rem); }
.running-head { display: flex; flex-direction: column; align-items: center; gap: 0.7rem; margin: -2rem 0 3rem; }
.running-head .chapter { font: italic 1.05rem/1.3 var(--font-serif); color: var(--note); text-align: center; }
.running-head .rule { width: 2.5rem; height: 2px; background: var(--ribbon); opacity: 0.8; }
.page-foot { margin-top: 3rem; text-align: right; font-size: 0.78rem; color: var(--note); }
.page :deep(.ProseMirror) {
  font-family: var(--font-text);
  font-size: 1.18rem;
  line-height: 1.8;
  hyphens: auto;
  color: var(--ink);
  caret-color: var(--ribbon);
  outline: none;
}
.page :deep(.ProseMirror p) { margin: 0; text-indent: 1.5em; }
.page :deep(.ProseMirror p:first-child) { text-indent: 0; }
/* capitular: a primeira letra da cena maior, como nos livros */
.page :deep(.ProseMirror > p:first-child::first-letter) {
  float: left;
  font-size: 3.4em;
  line-height: 0.85;
  padding: 0.08em 0.1em 0 0;
  color: var(--ink);
  font-family: var(--font-serif);
}
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

