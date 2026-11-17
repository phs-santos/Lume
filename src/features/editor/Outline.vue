<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { IconFilePlus, IconPlus, IconTrash } from "@tabler/icons-vue";
import { VueDraggable } from "vue-draggable-plus";
import { chapterCreate, chapterDelete, chapterRename, sceneCreate, sceneMove, sceneRename } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import { resolveDrop, singleScene } from "../../lib/outline";
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

/** Termina a edição. Capítulo de linha única renomeia também a cena, para os dois seguirem iguais. */
function finishRename(id: string, kind: "chapter" | "scene", alsoScene?: string) {
  if (editing.value !== id) return;
  const t = draftTitle.value.trim();
  editing.value = null;
  if (!t) return;
  if (kind === "scene") return run(() => sceneRename(id, t));
  run(async () => {
    await chapterRename(id, t);
    if (alsoScene) await sceneRename(alsoScene, t);
  });
}
</script>

<template>
  <nav class="outline" aria-label="Sumário do livro">

    <p v-if="error" class="error" role="alert">{{ error }}</p>

    <section v-for="c in outline.chapters" :key="c.chapter.id" :class="['chapter', { single: singleScene(c) }]">
      <div :class="['chapter-head', { current: singleScene(c)?.id === currentSceneId }]">
        <input
          v-if="editing === c.chapter.id"
          :id="`rename-${c.chapter.id}`"
          v-model="draftTitle"
          class="rename"
          @keydown.enter="finishRename(c.chapter.id, 'chapter', singleScene(c)?.id)"
          @keydown.esc="editing = null"
          @blur="finishRename(c.chapter.id, 'chapter', singleScene(c)?.id)"
        />
        <button
          v-else-if="singleScene(c)"
          class="chapter-btn"
          title="Clique duas vezes para renomear"
          @click="emit('select', singleScene(c)!.id)"
          @dblclick="startRename(c.chapter.id, c.chapter.title)"
        >
          {{ c.chapter.title }}
        </button>
        <h3 v-else title="Clique duas vezes para renomear" @dblclick="startRename(c.chapter.id, c.chapter.title)">
          {{ c.chapter.title }}
        </h3>
        <span class="count">{{ fmt(chapterWords(c.scenes)) }}</span>
        <button class="icon add-scene" title="Nova cena no fim do capítulo" aria-label="Nova cena no fim do capítulo" @click="addScene(c.chapter.id)"><IconPlus :size="15" stroke-width="2" /></button>
        <button class="icon" title="Apagar capítulo" aria-label="Apagar capítulo" @click="removeChapter(c.chapter.id)"><IconTrash :size="15" stroke-width="1.75" /></button>
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
          <button class="insert" :title="`Nova cena antes de ${s.title}`" :aria-label="`Inserir cena antes de ${s.title}`" @click="addScene(c.chapter.id, i)"><IconPlus :size="11" stroke-width="2.5" /></button>
          <input
            v-if="editing === s.id"
            :id="`rename-${s.id}`"
            v-model="draftTitle"
            class="rename scene-rename"
            @keydown.enter="finishRename(s.id, 'scene')"
            @keydown.esc="editing = null"
            @blur="finishRename(s.id, 'scene')"
          />
          <button v-else class="scene-btn" title="Clique duas vezes para renomear" @click="emit('select', s.id)" @dblclick="startRename(s.id, s.title)">
            <span class="scene-title">{{ s.title }}</span>
            <span class="count">{{ fmt(s.word_count) }}</span>
          </button>
        </li>
      </VueDraggable>
    </section>

    <button class="add chapter-add" @click="addChapter"><IconFilePlus :size="16" stroke-width="1.75" /> Novo capítulo</button>
  </nav>
</template>

<style scoped>
.outline {
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  scrollbar-width: none;
  padding: 1rem 0.75rem 3rem 1rem;
  background: var(--platen);
  color: var(--platen-ink);
  font-size: 0.9rem;
}
.outline::-webkit-scrollbar { display: none; }
/* o que rola some num degradê curto no topo, em vez de aparecer cortado pela metade */
.outline { -webkit-mask-image: linear-gradient(to bottom, transparent, #000 1rem); mask-image: linear-gradient(to bottom, transparent, #000 1rem); }
header {
  position: sticky;
  top: 0;
  z-index: 2;
  margin: 0 -0.75rem 1rem -1rem;
  padding: 1.25rem 0.75rem 0.9rem 1rem;
  background: var(--platen);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
h2 { font: 400 1.15rem/1.3 var(--font-mono); margin: 0; color: #fff; }
:root[data-theme="escuro"] h2 { color: var(--ink); }
.count { color: #8b919c; font-size: 0.78rem; font-variant-numeric: tabular-nums; }
.error { color: #ff8a80; margin: 0 0 1rem; }

.chapter { margin-bottom: 0.6rem; }
.chapter.single { margin-bottom: 0.1rem; }
/* capítulo de linha única: a própria linha abre a cena; a lista fica só como alvo de arrastar */
.chapter.single .scene { display: none; }
.chapter.single .scenes { min-height: 0.25rem; margin: 0; }
.chapter-head.current { border-left: 2px solid var(--ribbon); background: rgba(255, 255, 255, 0.07); border-radius: 0 6px 6px 0; }
.chapter-head.current .chapter-btn { color: #fff; }
:root[data-theme="escuro"] .chapter-head.current .chapter-btn { color: var(--ink); }
.chapter.single .chapter-head { padding: 0.35rem 0.25rem 0.35rem 0.4rem; border-left: 2px solid transparent; border-radius: 0 6px 6px 0; }
.chapter.single .chapter-head:hover { background: rgba(255, 255, 255, 0.05); }
.chapter.single .chapter-head.current { border-left-color: var(--ribbon); }
.chapter-btn { flex: 1; min-width: 0; padding: 0; border: 0; background: none; color: inherit; text-align: left; font-size: 0.85rem; font-weight: 600; cursor: pointer; }
.scene-rename { width: 100%; padding: 0.4rem 0.6rem 0.4rem 0.9rem; }
.chapter-head { display: flex; align-items: baseline; gap: 0.5rem; }
.chapter-head h3 { flex: 1; margin: 0; font-size: 0.85rem; font-weight: 600; cursor: default; }
.rename { flex: 1; background: transparent; border: 0; border-bottom: 1px solid #8b919c; padding: 0; color: inherit; }
.rename:focus { outline: none; }
.icon { display: inline-flex; background: none; border: 0; border-radius: 6px; color: #8b919c; cursor: pointer; opacity: 0; padding: 0.2rem; }
.icon:hover { color: #ff8a80; background: rgba(255, 255, 255, 0.06); }
.chapter-head:hover .icon, .icon:focus-visible { opacity: 1; }

.scenes { list-style: none; margin: 0.35rem 0 0; padding: 0; min-height: 0.75rem; }
.scene { position: relative; }
.scene-btn {
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  width: 100%;
  padding: 0.4rem 0.6rem 0.4rem 0.9rem;
  border: 0;
  border-radius: 0 6px 6px 0;
  border-left: 2px solid transparent;
  background: none;
  color: inherit;
  text-align: left;
  cursor: grab;
}
.scene-btn:hover { background: rgba(255, 255, 255, 0.05); }
.scene.current .scene-btn { border-left-color: var(--ribbon); color: #fff; background: rgba(255, 255, 255, 0.07); }
:root[data-theme="escuro"] .scene.current .scene-btn { color: var(--ink); }
.scene-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

/* "+" discreto entre cenas: aparece no hover, na borda de cima da cena */
.insert {
  position: absolute;
  left: 0.15rem;
  top: -0.5rem;
  display: grid;
  place-items: center;
  width: 1rem;
  height: 1rem;
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
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  margin: 0.3rem 0 0 0.5rem;
  padding: 0.3rem 0.6rem;
  border: 0;
  border-radius: 6px;
  background: none;
  color: #8b919c;
  cursor: pointer;
  font-size: 0.8rem;
}
.add:hover { color: #fff; background: rgba(255, 255, 255, 0.06); }
.icon.add-scene:hover { color: #fff; }
.chapter-add { margin: 0.5rem 0 0; width: 100%; justify-content: center; border: 1px dashed rgba(255, 255, 255, 0.15); }
:deep(.sortable-ghost) { opacity: 0.35; }
</style>
