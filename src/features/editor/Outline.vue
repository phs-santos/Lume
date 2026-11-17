<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { VueDraggable } from "vue-draggable-plus";
import { chapterCreate, chapterDelete, chapterRename, sceneCreate, sceneMove } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import { resolveDrop } from "../../lib/outline";
import type { Outline, SceneSummary } from "../../lib/types";

const props = defineProps<{ outline: Outline; currentSceneId: string }>();
const emit = defineEmits<{ select: [id: string]; changed: [] }>();

// Cópia local por capítulo: o arrastar mexe nela, e o recarregamento do pai a sincroniza.
const lists = ref<Record<string, SceneSummary[]>>({});
watch(
  () => props.outline,
  (o) => {
    lists.value = Object.fromEntries(o.chapters.map((c) => [c.chapter.id, [...c.scenes]]));
  },
  { immediate: true },
);

const error = ref("");
const editing = ref<string | null>(null);
const draftTitle = ref("");

const total = computed(() =>
  props.outline.chapters.reduce((n, c) => n + c.scenes.reduce((m, s) => m + s.word_count, 0), 0),
);
const chapterWords = (scenes: SceneSummary[]) => scenes.reduce((n, s) => n + s.word_count, 0);
const fmt = (n: number) => n.toLocaleString("pt-BR");

async function run(action: () => Promise<unknown>) {
  error.value = "";
  try {
    await action();
  } catch (e) {
    error.value = errorMessage(e);
  }
  emit("changed");
}

function onEnd(ev: { item: HTMLElement; to: HTMLElement; newIndex?: number }) {
  const sceneId = ev.item.dataset.id;
  const toChapterId = ev.to.dataset.chapterId;
  if (!sceneId || !toChapterId || ev.newIndex == null) return;
  const target = resolveDrop(props.outline, sceneId, toChapterId, ev.newIndex);
  if (!target) return;
  // Em caso de erro, `changed` recarrega o sumário e desfaz a mudança visual.
  run(() => sceneMove(sceneId, target.toChapterId, target.toIndex));
}

async function addScene(chapterId: string, index?: number) {
  error.value = "";
  try {
    const n = (lists.value[chapterId]?.length ?? 0) + 1;
    const s = await sceneCreate(chapterId, `Cena ${n}`, index);
    emit("changed");
    emit("select", s.id);
  } catch (e) {
    error.value = errorMessage(e);
  }
}

const addChapter = () =>
  run(() => chapterCreate(props.outline.book.id, `Capítulo ${props.outline.chapters.length + 1}`));

const removeChapter = (id: string) => run(() => chapterDelete(id));

async function startRename(id: string, title: string) {
  editing.value = id;
  draftTitle.value = title;
  await nextTick();
  (document.getElementById(`rename-${id}`) as HTMLInputElement | null)?.select();
}

function finishRename(id: string) {
  const t = draftTitle.value.trim();
  editing.value = null;
  if (t) run(() => chapterRename(id, t));
}
</script>

<template>
  <nav class="outline" aria-label="Sumário do livro">
    <header>
      <h2>{{ outline.book.title }}</h2>
      <p class="count">{{ fmt(total) }} palavras</p>
    </header>

    <p v-if="error" class="error" role="alert">{{ error }}</p>

    <section v-for="c in outline.chapters" :key="c.chapter.id" class="chapter">
      <div class="chapter-head">
        <input
          v-if="editing === c.chapter.id"
          :id="`rename-${c.chapter.id}`"
          v-model="draftTitle"
          class="rename"
          @keydown.enter="finishRename(c.chapter.id)"
          @keydown.esc="editing = null"
          @blur="finishRename(c.chapter.id)"
        />
        <h3 v-else :title="'Clique duas vezes para renomear'" @dblclick="startRename(c.chapter.id, c.chapter.title)">
          {{ c.chapter.title }}
        </h3>
        <span class="count">{{ fmt(chapterWords(c.scenes)) }}</span>
        <button class="icon" title="Apagar capítulo" aria-label="Apagar capítulo" @click="removeChapter(c.chapter.id)">×</button>
      </div>

      <VueDraggable
        v-model="lists[c.chapter.id]"
        tag="ol"
        class="scenes"
        group="scenes"
        :animation="120"
        :data-chapter-id="c.chapter.id"
        @end="onEnd"
      >
        <li
          v-for="(s, i) in lists[c.chapter.id]"
          :key="s.id"
          :data-id="s.id"
          :class="['scene', { current: s.id === currentSceneId }]"
        >
          <button class="insert" :aria-label="`Inserir cena antes de ${s.title}`" @click="addScene(c.chapter.id, i)">+</button>
          <button class="scene-btn" @click="emit('select', s.id)">
            <span class="scene-title">{{ s.title }}</span>
            <span class="count">{{ fmt(s.word_count) }}</span>
          </button>
        </li>
      </VueDraggable>
      <button class="add" @click="addScene(c.chapter.id)">+ cena</button>
    </section>

    <button class="add chapter-add" @click="addChapter">+ capítulo</button>
  </nav>
</template>

<style scoped>
.outline {
  height: 100%;
  overflow-y: auto;
  padding: 1.5rem 1rem 3rem 1.25rem;
  background: var(--platen);
  color: var(--platen-ink);
  font-size: 0.9rem;
}
header { margin-bottom: 1.5rem; }
h2 { font: 400 1.15rem/1.3 var(--font-mono); margin: 0; color: #fff; }
:root[data-theme="escuro"] h2 { color: var(--ink); }
.count { color: #8b919c; font-size: 0.78rem; font-variant-numeric: tabular-nums; }
.error { color: #ff8a80; margin: 0 0 1rem; }

.chapter { margin-bottom: 1.25rem; }
.chapter-head { display: flex; align-items: baseline; gap: 0.5rem; }
.chapter-head h3 { flex: 1; margin: 0; font-size: 0.85rem; font-weight: 600; cursor: default; }
.rename { flex: 1; background: transparent; border: 0; border-bottom: 1px solid #8b919c; padding: 0; color: inherit; }
.rename:focus { outline: none; }
.icon { background: none; border: 0; color: #8b919c; cursor: pointer; opacity: 0; padding: 0 0.2rem; }
.chapter-head:hover .icon, .icon:focus-visible { opacity: 1; }

.scenes { list-style: none; margin: 0.35rem 0 0; padding: 0; min-height: 0.75rem; }
.scene { position: relative; }
.scene-btn {
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  width: 100%;
  padding: 0.3rem 0.5rem 0.3rem 0.75rem;
  border: 0;
  border-left: 2px solid transparent;
  background: none;
  color: inherit;
  text-align: left;
  cursor: grab;
}
.scene-btn:hover { background: rgba(255, 255, 255, 0.05); }
.scene.current .scene-btn { border-left-color: var(--ribbon); color: #fff; }
:root[data-theme="escuro"] .scene.current .scene-btn { color: var(--ink); }
.scene-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

/* "+" discreto entre cenas: aparece no hover, na borda de cima da cena */
.insert {
  position: absolute;
  left: -0.95rem;
  top: -0.55rem;
  width: 1.1rem;
  height: 1.1rem;
  line-height: 1rem;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: var(--ribbon);
  color: #fff;
  font-size: 0.8rem;
  cursor: pointer;
  opacity: 0;
  z-index: 1;
}
.scene:hover .insert, .insert:focus-visible { opacity: 1; }

.add {
  margin: 0.25rem 0 0 0.75rem;
  padding: 0;
  border: 0;
  background: none;
  color: #8b919c;
  cursor: pointer;
  font-size: 0.8rem;
}
.add:hover { color: inherit; }
.chapter-add { margin-left: 0; }
:deep(.sortable-ghost) { opacity: 0.35; }
</style>
