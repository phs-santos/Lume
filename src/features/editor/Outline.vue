<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { IconDots, IconFilePlus, IconPlus } from "@tabler/icons-vue";
import { VueDraggable } from "vue-draggable-plus";
import { chapterCreate, chapterDelete, chapterRename, sceneCreate, sceneDelete, sceneMove, sceneRename } from "../../lib/api";
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

/** Menu "⋯" de cada linha (também no clique direito): diz se é capítulo ou cena e reúne as ações. */
interface MenuItem { label: string; run: () => void; danger?: boolean }
const menu = ref<{ key: string; kind: string; items: MenuItem[]; y: number } | null>(null);
const confirming = ref<string | null>(null);

function openMenu(ev: MouseEvent, key: string, kind: string, items: MenuItem[]) {
  confirming.value = null;
  if (menu.value?.key === key && ev.type === "click") return void (menu.value = null);
  const nav = (ev.currentTarget as HTMLElement).closest(".outline") as HTMLElement;
  const box = nav.getBoundingClientRect();
  const row = (ev.currentTarget as HTMLElement).getBoundingClientRect();
  // abre logo abaixo da linha, dentro do sumário
  menu.value = { key, kind, items, y: row.bottom - box.top + nav.scrollTop + 2 };
}

/** Ações que apagam pedem um segundo clique. */
function pick(item: MenuItem) {
  if (item.danger && confirming.value !== item.label) return void (confirming.value = item.label);
  menu.value = null;
  confirming.value = null;
  item.run();
}

const closeMenu = (e: Event) => {
  if (!(e.target as HTMLElement).closest?.(".menu, .more")) menu.value = null;
};
const escMenu = (e: KeyboardEvent) => e.key === "Escape" && (menu.value = null);
onMounted(() => {
  document.addEventListener("mousedown", closeMenu);
  document.addEventListener("keydown", escMenu);
});
onBeforeUnmount(() => {
  document.removeEventListener("mousedown", closeMenu);
  document.removeEventListener("keydown", escMenu);
});

function chapterItems(chapterId: string, title: string): MenuItem[] {
  return [
    { label: "Nova cena no fim do capítulo", run: () => addScene(chapterId) },
    { label: "Renomear capítulo", run: () => startRename(chapterId, title) },
    { label: "Apagar capítulo", danger: true, run: () => removeChapter(chapterId) },
  ];
}

function sceneItems(chapterId: string, s: SceneSummary, i: number, single: boolean): MenuItem[] {
  return [
    { label: "Nova cena depois desta", run: () => addScene(chapterId, i + 1) },
    { label: "Nova cena antes desta", run: () => addScene(chapterId, i) },
    { label: single ? "Renomear" : "Renomear cena", run: () => startRename(s.id, s.title) },
    single
      ? { label: "Apagar capítulo", danger: true, run: () => removeChapter(chapterId) }
      : { label: "Apagar cena", danger: true, run: () => run(() => sceneDelete(s.id)) },
  ];
}

async function startRename(id: string, title: string) {
  editing.value = id;
  draftTitle.value = title;
  await nextTick();
  (document.getElementById(`rename-${id}`) as HTMLInputElement | null)?.select();
}

/**
 * Termina a edição de `key` (id do capítulo ou da cena). Renomeia o capítulo e/ou a cena dados:
 * na linha única os dois mudam juntos, para continuarem iguais.
 */
function finishRename(key: string, chapterId: string | null, sceneId: string | null) {
  if (editing.value !== key) return;
  const t = draftTitle.value.trim();
  editing.value = null;
  if (!t) return;
  run(async () => {
    if (chapterId) await chapterRename(chapterId, t);
    if (sceneId) await sceneRename(sceneId, t);
  });
}
</script>

<template>
  <nav class="outline" aria-label="Sumário do livro">

    <p v-if="error" class="error" role="alert">{{ error }}</p>

    <section v-for="c in outline.chapters" :key="c.chapter.id" :class="['chapter', { single: singleScene(c) }]">
      <div v-if="!singleScene(c)" class="chapter-head hover-zone">
        <input
          v-if="editing === c.chapter.id"
          :id="`rename-${c.chapter.id}`"
          v-model="draftTitle"
          class="rename"
          @keydown.enter="finishRename(c.chapter.id, c.chapter.id, null)"
          @keydown.esc="editing = null"
          @blur="finishRename(c.chapter.id, c.chapter.id, null)"
        />
        <h3 v-else title="Clique duas vezes para renomear" @dblclick="startRename(c.chapter.id, c.chapter.title)">
          {{ c.chapter.title }}
        </h3>
        <span class="count">{{ fmt(chapterWords(c.scenes)) }}</span>
        <button class="icon add-scene" title="Nova cena no fim do capítulo" aria-label="Nova cena no fim do capítulo" @click="addScene(c.chapter.id)"><IconPlus :size="15" stroke-width="2" /></button>
        <button
          class="more"
          title="Mais ações do capítulo"
          aria-label="Mais ações do capítulo"
          :aria-expanded="menu?.key === c.chapter.id"
          @click="openMenu($event, c.chapter.id, 'Capítulo', chapterItems(c.chapter.id, c.chapter.title))"
        ><IconDots :size="15" stroke-width="2" /></button>
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
            @keydown.enter="finishRename(s.id, singleScene(c) ? c.chapter.id : null, s.id)"
            @keydown.esc="editing = null"
            @blur="finishRename(s.id, singleScene(c) ? c.chapter.id : null, s.id)"
          />
          <div
            v-else
            class="row hover-zone"
            @contextmenu.prevent="openMenu($event, s.id, singleScene(c) ? 'Capítulo de uma cena' : 'Cena', sceneItems(c.chapter.id, s, i, !!singleScene(c)))"
          >
            <button
              class="scene-btn"
              title="Clique duas vezes (ou F2) para renomear"
              @click="emit('select', s.id)"
              @dblclick="startRename(s.id, s.title)"
              @keydown.f2.prevent="startRename(s.id, s.title)"
            >
              <span class="scene-title">{{ s.title }}</span>
              <span class="count">{{ fmt(s.word_count) }}</span>
            </button>
            <button v-if="singleScene(c)" class="icon add-scene" title="Nova cena depois desta" aria-label="Nova cena no fim do capítulo" @click="addScene(c.chapter.id)"><IconPlus :size="15" stroke-width="2" /></button>
            <button
              class="more"
              :title="singleScene(c) ? 'Mais ações do capítulo' : 'Mais ações da cena'"
              :aria-label="singleScene(c) ? 'Mais ações do capítulo' : 'Mais ações da cena'"
              :aria-expanded="menu?.key === s.id"
              @click="openMenu($event, s.id, singleScene(c) ? 'Capítulo de uma cena' : 'Cena', sceneItems(c.chapter.id, s, i, !!singleScene(c)))"
            ><IconDots :size="15" stroke-width="2" /></button>
          </div>
        </li>
      </VueDraggable>
    </section>

    <div v-if="menu" class="menu" role="menu" :style="{ top: `${menu.y}px` }">
      <p class="menu-kind">{{ menu.kind }}</p>
      <button
        v-for="it in menu.items"
        :key="it.label"
        role="menuitem"
        :class="['menu-item', { danger: it.danger, confirm: confirming === it.label }]"
        @click="pick(it)"
      >{{ confirming === it.label ? `Confirmar: ${it.label.toLowerCase()}` : it.label }}</button>
    </div>

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
.outline { position: relative; }
/* "⋯": sempre visível, discreto; mais forte no hover */
.more { display: inline-flex; flex: none; padding: 0.2rem; border: 0; border-radius: 6px; background: none; color: #8b919c; opacity: 0.55; cursor: pointer; }
.more:hover, .more[aria-expanded="true"], .hover-zone:hover .more, .more:focus-visible { opacity: 1; color: #fff; background: rgba(255, 255, 255, 0.06); }
.row .more { margin-right: 0.3rem; }
.menu {
  position: absolute;
  right: 0.75rem;
  z-index: 20;
  display: flex;
  flex-direction: column;
  min-width: 12rem;
  padding: 0.3rem;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 10px;
  background: #262b36;
  box-shadow: 0 16px 32px -12px rgba(0, 0, 0, 0.6);
}
.menu-kind { margin: 0.15rem 0.5rem 0.3rem; font-size: 0.68rem; text-transform: uppercase; letter-spacing: 0.06em; color: #8b919c; }
.menu-item { padding: 0.4rem 0.5rem; border: 0; border-radius: 6px; background: none; color: #e6e8ec; text-align: left; font-size: 0.82rem; cursor: pointer; }
.menu-item:hover, .menu-item:focus-visible { background: rgba(255, 255, 255, 0.08); }
.menu-item.danger { color: #ff8a80; }
.menu-item.confirm { background: rgba(255, 138, 128, 0.14); }
/* o que rola some num degradê curto no topo, em vez de aparecer cortado pela metade */
.outline { -webkit-mask-image: linear-gradient(to bottom, transparent, #000 1rem); mask-image: linear-gradient(to bottom, transparent, #000 1rem); }
.count { color: #8b919c; font-size: 0.78rem; font-variant-numeric: tabular-nums; }
.error { color: #ff8a80; margin: 0 0 1rem; }

.chapter { margin-bottom: 0.6rem; }
/* capítulo de cena única: uma linha só, que é a própria cena (abre, arrasta e solta como cena) */
.chapter.single { margin-bottom: 0.1rem; }
.chapter.single .scenes { margin: 0; min-height: 0; }
.chapter.single .scene-btn { padding-left: 0.4rem; font-size: 0.85rem; font-weight: 600; }
.chapter.single .insert { display: none; }
.scene-rename { width: 100%; padding: 0.4rem 0.6rem 0.4rem 0.9rem; }
.chapter-head { display: flex; align-items: center; gap: 0.35rem; padding-right: 0.6rem; min-height: 1.6rem; }
.chapter-head h3 { flex: 1; margin: 0; font-size: 0.85rem; font-weight: 600; cursor: default; }
.rename { flex: 1; background: transparent; border: 0; border-bottom: 1px solid #8b919c; padding: 0; color: inherit; }
.rename:focus { outline: none; }
.icon { display: inline-flex; flex: none; background: none; border: 0; border-radius: 6px; color: #8b919c; cursor: pointer; padding: 0.2rem; }
.icon:hover { color: #ff8a80; background: rgba(255, 255, 255, 0.06); }
.icon.add-scene:hover { color: #fff; }
/* ícones só no hover ou no foco do teclado, no lugar da contagem. Escondidos com largura 0 (não
   display:none), para o Tab ainda chegar neles. */
.hover-zone .icon { width: 0; padding: 0; overflow: hidden; opacity: 0; }
.hover-zone:hover .icon, .hover-zone:focus-within .icon { width: auto; padding: 0.2rem; opacity: 1; }
.hover-zone:hover .count, .hover-zone:focus-within .count { display: none; }
.row { display: flex; align-items: center; border-left: 2px solid transparent; border-radius: 0 6px 6px 0; }
.row:hover { background: rgba(255, 255, 255, 0.05); }
.chapter.single .row .icon:last-child { margin-right: 0.4rem; }

.scenes { list-style: none; margin: 0.35rem 0 0; padding: 0; min-height: 0.75rem; }
.scene { position: relative; }
.scene-btn {
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  width: 100%;
  padding: 0.4rem 0.6rem 0.4rem 0.9rem;
  border: 0;
  background: none;
  color: inherit;
  text-align: left;
  cursor: grab;
}
.scene-btn { flex: 1; min-width: 0; }
.scene.current .row { border-left-color: var(--ribbon); background: rgba(255, 255, 255, 0.07); }
.scene.current .scene-btn { color: #fff; }
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
.chapter-add { margin: 0.5rem 0 0; width: 100%; justify-content: center; border: 1px dashed rgba(255, 255, 255, 0.15); }
:deep(.sortable-ghost) { opacity: 0.35; }
</style>
