<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { IconGripVertical, IconSearch } from "@tabler/icons-vue";
import { VueDraggable } from "vue-draggable-plus";
import { bibleAbsorbNote, bibleDelete, bibleList, bibleUpdate } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import type { BibleEntry, BibleKind } from "../../lib/types";

const props = defineProps<{ bookId: string; sceneTitles: Record<string, string> }>();
const emit = defineEmits<{ openScene: [id: string] }>();

const GROUPS: { kind: BibleKind; label: string }[] = [
  { kind: "personagem", label: "Personagens" },
  { kind: "lugar", label: "Lugares" },
  { kind: "fato", label: "Fatos" },
  { kind: "linha_do_tempo", label: "Linha do tempo" },
];

const entries = ref<BibleEntry[]>([]);
const query = ref("");
const open = ref<string | null>(null);
const confirmDelete = ref<string | null>(null);
const draft = ref({ name: "", aliases: "", body: "" });
const error = ref("");
const loaded = ref(false);

const fold = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

async function load() {
  try {
    entries.value = await bibleList(props.bookId);
  } catch (e) {
    error.value = errorMessage(e);
  }
  loaded.value = true;
}
onMounted(load);

const matches = (e: BibleEntry) => {
  const q = fold(query.value.trim());
  return !q || [e.name, e.body, ...e.aliases].some((t) => fold(t).includes(q));
};

const groups = computed(() =>
  GROUPS.map((g) => {
    const list = entries.value.filter((e) => e.kind === g.kind && matches(e));
    list.sort((a, b) =>
      g.kind === "linha_do_tempo" ? (a.story_order ?? 1e9) - (b.story_order ?? 1e9) : a.name.localeCompare(b.name, "pt-BR"),
    );
    return { ...g, list };
  }).filter((g) => g.list.length),
);

/** Cópia da linha do tempo para o arrastar (o draggable mexe nela). */
const timeline = ref<BibleEntry[]>([]);
const timelineView = computed({
  get: () => groups.value.find((g) => g.kind === "linha_do_tempo")?.list ?? [],
  set: (v) => (timeline.value = v),
});

function toggle(e: BibleEntry) {
  confirmDelete.value = null;
  if (open.value === e.id) return void (open.value = null);
  open.value = e.id;
  draft.value = { name: e.name, aliases: e.aliases.join(", "), body: e.body };
}

function replace(e: BibleEntry) {
  entries.value = entries.value.map((x) => (x.id === e.id ? e : x));
}

async function save(e: BibleEntry) {
  error.value = "";
  const aliases = draft.value.aliases.split(",").map((a) => a.trim()).filter(Boolean);
  try {
    replace(await bibleUpdate(e.id, draft.value.name.trim(), aliases, draft.value.body, e.story_order));
    open.value = null;
  } catch (err) {
    error.value = errorMessage(err);
  }
}

async function absorb(e: BibleEntry, i: number) {
  try {
    const updated = await bibleAbsorbNote(e.id, i);
    replace(updated);
    draft.value.body = updated.body;
  } catch (err) {
    error.value = errorMessage(err);
  }
}

async function remove(e: BibleEntry) {
  if (confirmDelete.value !== e.id) return void (confirmDelete.value = e.id);
  try {
    await bibleDelete(e.id);
    entries.value = entries.value.filter((x) => x.id !== e.id);
    open.value = null;
  } catch (err) {
    error.value = errorMessage(err);
  }
}

/** Grava a nova ordem da linha do tempo (só o que mudou de lugar). */
async function reorderTimeline(ids: string[]) {
  try {
    for (const [i, id] of ids.entries()) {
      const e = entries.value.find((x) => x.id === id);
      if (!e || e.story_order === i + 1) continue;
      replace(await bibleUpdate(e.id, e.name, e.aliases, e.body, i + 1));
      replace({ ...entries.value.find((x) => x.id === id)!, story_order: i + 1 });
    }
  } catch (err) {
    error.value = errorMessage(err);
  }
}

const onTimelineEnd = () => reorderTimeline(timeline.value.map((e) => e.id));

defineExpose({ reorderTimeline, reload: load });
</script>

<template>
  <div class="bible">
    <label class="search">
      <IconSearch :size="14" stroke-width="2" />
      <input v-model="query" type="search" placeholder="Buscar fichas" aria-label="Buscar fichas" />
    </label>
    <p v-if="error" class="err" role="alert">{{ error }}</p>

    <div class="list">
      <p v-if="loaded && !entries.length" class="empty">
        A Bíblia ainda está vazia. Ela se preenche sozinha enquanto você escreve e conversa com o assistente.
      </p>
      <p v-else-if="loaded && !groups.length" class="empty">Nada encontrado.</p>

      <section v-for="g in groups" :key="g.kind" class="group">
        <h4>{{ g.label }} <span>{{ g.list.length }}</span></h4>
        <component
          :is="g.kind === 'linha_do_tempo' && !query ? VueDraggable : 'ul'"
          v-bind="g.kind === 'linha_do_tempo' && !query ? { modelValue: timelineView, 'onUpdate:modelValue': (v: BibleEntry[]) => (timelineView = v), tag: 'ul', handle: '.grip', animation: 120, onEnd: onTimelineEnd } : {}"
          class="entries"
        >
          <li v-for="e in g.list" :key="e.id" :data-id="e.id" :class="['entry', { open: open === e.id }]">
            <div class="entry-head" role="button" tabindex="0" @click="toggle(e)" @keydown.enter.prevent="toggle(e)">
              <IconGripVertical v-if="g.kind === 'linha_do_tempo' && !query" class="grip" :size="13" @click.stop />
              <div class="entry-main">
                <span class="entry-name">{{ e.name }}</span>
                <span v-if="e.aliases.length" class="aliases">{{ e.aliases.join(", ") }}</span>
                <span v-if="open !== e.id" class="preview">{{ e.body.split("\n")[0] }}</span>
              </div>
              <span v-if="e.ai_notes.length" class="dot" title="A IA tem fatos novos para esta ficha" />
            </div>

            <form v-if="open === e.id" class="edit" @submit.prevent="save(e)">
              <input v-model="draft.name" name="name" aria-label="Nome" />
              <input v-model="draft.aliases" name="aliases" placeholder="Apelidos, separados por vírgula" aria-label="Apelidos" />
              <textarea v-model="draft.body" rows="4" aria-label="Texto da ficha" />
              <div v-if="e.ai_notes.length" class="notes">
                <p class="notes-label">Fatos novos que a IA achou</p>
                <div v-for="(n, i) in e.ai_notes" :key="i" class="note">
                  <span>{{ n.fact }}</span>
                  <button type="button" class="link" @click="absorb(e, i)">incorporar</button>
                </div>
              </div>
              <button v-if="e.source_scene_id" type="button" class="link source" data-test="source" @click="emit('openScene', e.source_scene_id)">
                de: {{ sceneTitles[e.source_scene_id] ?? "cena" }}
              </button>
              <div class="actions">
                <button class="btn small" data-test="save" type="submit" :disabled="!draft.name.trim()">Salvar</button>
                <button type="button" class="link" @click="open = null">Cancelar</button>
                <button type="button" :class="['link', 'del', { confirm: confirmDelete === e.id }]" @click="remove(e)">
                  {{ confirmDelete === e.id ? "Confirmar exclusão" : "Apagar" }}
                </button>
              </div>
            </form>
          </li>
        </component>
      </section>
    </div>
  </div>
</template>

<style scoped>
.bible { display: flex; flex-direction: column; min-height: 0; height: 100%; }
.search { display: flex; align-items: center; gap: 0.4rem; margin: 0 0.9rem 0.5rem; padding: 0.35rem 0.6rem; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 8px; color: #8b919c; }
.search input { flex: 1; min-width: 0; border: 0; background: none; color: var(--platen-ink); font-size: 0.84rem; outline: none; }
.list { flex: 1; min-height: 0; overflow-y: auto; padding: 0 0.6rem 1rem 0.9rem; scrollbar-width: thin; scrollbar-color: rgba(255, 255, 255, 0.15) transparent; }
.empty { color: #8b919c; font-size: 0.82rem; }
.err { margin: 0 0.9rem 0.5rem; font-size: 0.8rem; color: #ff8a80; }
.group { margin-bottom: 0.8rem; }
h4 { margin: 0 0 0.3rem; font-size: 0.72rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: #8b919c; }
h4 span { font-weight: 400; }
.entries { list-style: none; margin: 0; padding: 0; }
.entry { border-radius: 8px; }
.entry.open { background: rgba(255, 255, 255, 0.05); }
.entry-head { display: flex; align-items: flex-start; gap: 0.35rem; padding: 0.4rem 0.5rem; border-radius: 8px; cursor: pointer; }
.entry-head:hover { background: rgba(255, 255, 255, 0.05); }
.entry-main { display: flex; flex-direction: column; min-width: 0; flex: 1; }
.entry-name { font-size: 0.86rem; font-weight: 600; color: #fff; }
:root[data-theme="escuro"] .entry-name { color: var(--ink); }
.aliases { font-size: 0.74rem; color: #8b919c; }
.preview { font-size: 0.8rem; color: #aeb3bc; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.grip { flex: none; margin-top: 0.2rem; color: #8b919c; cursor: grab; }
.dot { flex: none; width: 6px; height: 6px; margin-top: 0.45rem; border-radius: 50%; background: var(--ribbon); }
.edit { display: flex; flex-direction: column; gap: 0.4rem; padding: 0 0.5rem 0.6rem; }
.edit input, .edit textarea { width: 100%; padding: 0.35rem 0.5rem; border: 1px solid rgba(255, 255, 255, 0.12); border-radius: 6px; background: rgba(0, 0, 0, 0.18); color: var(--platen-ink); font-size: 0.82rem; resize: vertical; }
.notes { padding: 0.4rem 0.5rem; border-left: 2px solid var(--ribbon); background: rgba(255, 255, 255, 0.03); }
.notes-label { margin: 0 0 0.25rem; font-size: 0.72rem; color: #8b919c; }
.note { display: flex; justify-content: space-between; gap: 0.5rem; font-size: 0.8rem; }
.note .link, .source { font-size: 0.76rem; color: #aeb3bc; }
.source { align-self: flex-start; padding: 0; }
.actions { display: flex; align-items: center; gap: 0.6rem; }
.btn.small { padding: 0.3rem 0.7rem; font-size: 0.78rem; }
.actions .link { font-size: 0.78rem; color: #aeb3bc; }
.del { margin-left: auto; }
.del.confirm { color: #ff8a80; }
</style>
