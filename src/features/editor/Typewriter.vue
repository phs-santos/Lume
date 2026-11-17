<script setup lang="ts">
import { EditorContent, useEditor } from "@tiptap/vue-3";
import { onBeforeUnmount, ref } from "vue";
import { countWords } from "../../lib/wordcount";
import { editorOptions } from "./editorConfig";
import { playKey } from "./keySound";

const props = defineProps<{ sceneId: string; contentJson: string; focusMode: boolean; keySound: boolean; machine: boolean }>();
const emit = defineEmits<{ change: [sceneId: string, json: string, words: number] }>();
const scroller = ref<HTMLElement | null>(null);

function parse(json: string) {
  try {
    return JSON.parse(json);
  } catch {
    return "";
  }
}

/** Altura da linha de datilografia (fração da tela). Com a máquina, logo acima do rolo. */
const LINE_AT = { machine: 0.64, plain: 0.5 };

/** Mantém a linha do cursor fixa, como o rolo de uma máquina de escrever. */
function centerCaret() {
  const ed = editor.value;
  const box = scroller.value;
  if (!ed || !box) return;
  const caret = ed.view.coordsAtPos(ed.state.selection.head);
  const rect = box.getBoundingClientRect();
  const lineBottom = rect.top + rect.height * (props.machine ? LINE_AT.machine : LINE_AT.plain);
  box.scrollTop += caret.bottom - lineBottom;
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
  <div :class="['typewriter', { focus: focusMode, 'with-machine': machine }]" @wheel.passive="forwardWheel">
    <div ref="scroller" class="sheet">
      <EditorContent :editor="editor" class="page" />
    </div>
    <!-- A máquina: o rolo logo abaixo da linha que se escreve e o corpo com as teclas. Só decoração. -->
    <div v-if="machine" class="machine" aria-hidden="true">
      <div class="roller"><span class="knob left" /><span class="knob right" /></div>
      <div class="body">
        <span class="spool left" /><span class="spool right" />
        <div class="keys" />
        <div class="keys second" />
        <div class="spacebar" />
      </div>
    </div>
  </div>
</template>

<style scoped>
/* A mesa (madeira) fica parada; só a folha rola. */
.typewriter {
  --line: 64%;
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
  background: var(--sheet);
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.08), 0 30px 60px -20px rgba(0, 0, 0, 0.55);
}
/* Com a máquina, a barra de rolagem não aparece: o rolo é que "puxa" o papel. */
.with-machine .sheet { scrollbar-width: none; }
.sheet:not(:hover) { scrollbar-color: transparent transparent; }
/* meia tela de folga em cima e embaixo: a primeira e a última linha também chegam ao centro */
.page { padding: 50vh clamp(1.5rem, 6vw, 4.5rem); }
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

/* ---------- a máquina ---------- */
.machine {
  --w: min(100% - 1rem, calc(68ch + 9rem + 7rem));
  position: absolute;
  left: 0;
  right: 0;
  top: calc(var(--line) + 0.35rem);
  bottom: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  pointer-events: none;
}
.roller {
  position: relative;
  width: var(--w);
  height: 2.6rem;
  border-radius: 1.3rem;
  background: linear-gradient(180deg, #4a4e57 0%, #1d1f24 45%, #0f1013 70%, #2a2d33 100%);
  box-shadow: 0 6px 14px rgba(0, 0, 0, 0.45);
  z-index: 2;
}
.knob {
  position: absolute;
  top: 50%;
  width: 3.6rem;
  height: 3.6rem;
  margin-top: -1.8rem;
  border-radius: 50%;
  background:
    repeating-conic-gradient(from 0deg, rgba(255, 255, 255, 0.08) 0 6deg, transparent 6deg 12deg),
    radial-gradient(circle, #3b3f47 0 55%, #15161a 56% 100%);
  box-shadow: 0 6px 14px rgba(0, 0, 0, 0.5);
}
.knob.left { left: -2.4rem; }
.knob.right { right: -2.4rem; }
.body {
  position: relative;
  flex: 1;
  width: calc(var(--w) + 3rem);
  margin-top: -1.1rem;
  padding: 2.6rem 3.5rem 0;
  border-radius: 2.5rem 2.5rem 0 0;
  background: linear-gradient(180deg, #2c2f36 0%, #1c1e22 60%, #151619 100%);
  box-shadow: inset 0 2px 0 rgba(255, 255, 255, 0.06), 0 -10px 30px rgba(0, 0, 0, 0.35);
}
.spool {
  position: absolute;
  top: 1.1rem;
  width: 3.2rem;
  height: 3.2rem;
  border-radius: 50%;
  background: radial-gradient(circle, #15161a 0 18%, #b42318 19% 46%, #0d0d0f 47% 70%, #3a3d45 71%);
}
.spool.left { left: 2.2rem; }
.spool.right { right: 2.2rem; }
.keys {
  height: 2.6rem;
  margin: 1.6rem auto 0;
  width: 78%;
  background: radial-gradient(circle at 50% 45%, #e9e4d8 0 30%, #8f8a80 33% 38%, #0e0f12 40% 47%, transparent 49%);
  background-size: 2.6rem 2.6rem;
  background-repeat: space no-repeat;
  opacity: 0.92;
}
.keys.second { width: 70%; margin-top: 0.3rem; }
.spacebar {
  width: 38%;
  height: 0.9rem;
  margin: 0.8rem auto 0;
  border-radius: 0.45rem;
  background: linear-gradient(180deg, #e9e4d8, #a9a397);
  box-shadow: 0 2px 0 #0e0f12;
}

.focus .page :deep(.ProseMirror > *) { opacity: 0.3; transition: opacity 0.2s; }
.focus .page :deep(.ProseMirror > .is-current) { opacity: 1; }
</style>

