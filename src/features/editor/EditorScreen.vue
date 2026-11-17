<script setup lang="ts">
import { IconAdjustmentsHorizontal, IconArrowLeft, IconDownload, IconHistory, IconLayoutSidebarLeftCollapse, IconLayoutSidebarLeftExpand } from "@tabler/icons-vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { toDocx } from "../../lib/export/docx";
import { toMarkdown } from "../../lib/export/markdown";
import type { ExportBook } from "../../lib/export/types";
import { onBeforeUnmount, onMounted, ref } from "vue";
import { bookOutline, sceneGet, sceneRename, sceneRestore, sceneSave, sceneVersions } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import type { Outline as OutlineT, Scene, Version } from "../../lib/types";
import Outline from "./Outline.vue";
import SettingsPanel from "./SettingsPanel.vue";
import Typewriter from "./Typewriter.vue";
import { loadPrefs, prefs } from "./settings";
import { useAutosave } from "./useAutosave";
import { handleCloseRequest, registerFlusher } from "./exitGuard";

const props = defineProps<{ bookId: string }>();
const emit = defineEmits<{ close: [] }>();

const outline = ref<OutlineT | null>(null);
const scene = ref<Scene | null>(null);
const sidebar = ref(true);
const panel = ref<"none" | "settings" | "versions" | "export">("none");
const exportMsg = ref("");
const versions = ref<Version[]>([]);
const error = ref("");
const saveState = ref<"salvo" | "salvando" | "erro">("salvo");
const editorKey = ref(0); // força recriar o editor ao restaurar uma versão

const autosave = useAutosave(async (sceneId, json, words) => {
  saveState.value = "salvando";
  try {
    await sceneSave(sceneId, json, words);
    const s = scene.value;
    if (s?.id === sceneId) {
      s.content_json = json;
      s.word_count = words;
    }
    for (const c of outline.value?.chapters ?? [])
      for (const x of c.scenes) if (x.id === sceneId) x.word_count = words;
    saveState.value = "salvo";
  } catch (e) {
    saveState.value = "erro";
    throw e;
  }
});

/** Cada alteração leva o id da cena de onde veio: nunca grava texto numa cena errada. */
function onChange(sceneId: string, json: string, words: number) {
  autosave.schedule(sceneId, json, words);
}

async function flushSafely() {
  try {
    await autosave.flush();
    return true;
  } catch (e) {
    error.value = `Não consegui salvar: ${errorMessage(e)}`;
    return false;
  }
}

async function reloadOutline() {
  outline.value = await bookOutline(props.bookId);
}

async function openScene(id: string) {
  if (scene.value?.id === id) return;
  if (!(await flushSafely())) return;
  scene.value = await sceneGet(id);
  panel.value = "none";
}

async function renameScene(e: Event) {
  const t = (e.target as HTMLInputElement).value.trim();
  if (!scene.value || !t || t === scene.value.title) return;
  await sceneRename(scene.value.id, t);
  scene.value.title = t;
  await reloadOutline();
}

async function showVersions() {
  if (panel.value === "versions") return void (panel.value = "none");
  await flushSafely();
  if (scene.value) versions.value = await sceneVersions(scene.value.id);
  panel.value = "versions";
}

async function restore(v: Version) {
  if (!(await flushSafely())) return;
  scene.value = await sceneRestore(v.id);
  editorKey.value++;
  panel.value = "none";
  await reloadOutline();
}

async function exportBook(format: "md" | "docx") {
  exportMsg.value = "";
  if (!outline.value || !(await flushSafely())) return;
  try {
    const o = await bookOutline(props.bookId);
    const book: ExportBook = {
      title: o.book.title,
      chapters: await Promise.all(
        o.chapters.map(async (c) => ({
          title: c.chapter.title,
          scenes: await Promise.all(
            c.scenes.map(async (s) => ({ title: s.title, doc: JSON.parse((await sceneGet(s.id)).content_json) })),
          ),
        })),
      ),
    };
    const name = o.book.title.replace(/[\\/:*?"<>|]/g, "-");
    const path = await saveDialog({
      defaultPath: `${name}.${format}`,
      filters: [format === "md" ? { name: "Markdown", extensions: ["md"] } : { name: "Word", extensions: ["docx"] }],
    });
    if (!path) return;
    const bytes = format === "md" ? new TextEncoder().encode(toMarkdown(book)) : await toDocx(book);
    await writeFile(path, bytes);
    exportMsg.value = `Exportado para ${path}`;
  } catch (e) {
    exportMsg.value = `Não consegui exportar: ${errorMessage(e)}`;
  }
}

async function back() {
  if (await flushSafely()) emit("close");
}

function onKey(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === "\\") {
    e.preventDefault();
    sidebar.value = !sidebar.value;
  }
}

const when = (t: number) =>
  new Date(t).toLocaleString("pt-BR", { day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit" });

let unlistenClose: (() => void) | null = null;
const unregisterFlusher = registerFlusher(flushSafely);
const flushOnBlur = () => void flushSafely();

onMounted(async () => {
  window.addEventListener("keydown", onKey);
  window.addEventListener("blur", flushOnBlur);
  try {
    await loadPrefs();
    await reloadOutline();
    const first = outline.value?.chapters.flatMap((c) => c.scenes)[0];
    if (first) scene.value = await sceneGet(first.id);
  } catch (e) {
    error.value = errorMessage(e);
  }
  // Antes de a janela fechar, grava o pendente; se não conseguir, a janela não fecha.
  unlistenClose = await getCurrentWindow().onCloseRequested(handleCloseRequest);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKey);
  window.removeEventListener("blur", flushOnBlur);
  unlistenClose?.();
  unregisterFlusher();
});
</script>

<template>
  <div :class="['editor', { 'no-sidebar': !sidebar }]">
    <Outline
      v-if="outline && sidebar"
      :outline="outline"
      :current-scene-id="scene?.id ?? ''"
      @select="openScene"
      @changed="reloadOutline"
    />

    <main class="desk">
      <header class="bar">
        <button class="link" title="Voltar aos livros" @click="back"><IconArrowLeft :size="18" stroke-width="1.75" /> Livros</button>
        <button class="link icon" :title="sidebar ? 'Esconder sumário (Cmd+\\)' : 'Mostrar sumário (Cmd+\\)'" :aria-label="sidebar ? 'Esconder sumário' : 'Mostrar sumário'" @click="sidebar = !sidebar">
          <IconLayoutSidebarLeftCollapse v-if="sidebar" :size="18" stroke-width="1.75" />
          <IconLayoutSidebarLeftExpand v-else :size="18" stroke-width="1.75" />
        </button>
        <input
          v-if="scene"
          class="scene-title"
          :value="scene.title"
          aria-label="Título da cena"
          @blur="renameScene"
          @keydown.enter="($event.target as HTMLInputElement).blur()"
        />
        <span class="status" :class="saveState" role="status">
          <span class="dot" />{{ saveState === "erro" ? "Não salvo" : saveState === "salvando" ? "Salvando…" : "Salvo" }}
        </span>
        <nav class="tools" aria-label="Ferramentas">
          <button :class="['link', { on: panel === 'versions' }]" @click="showVersions"><IconHistory :size="18" stroke-width="1.75" /> Versões</button>
          <button :class="['link', { on: panel === 'export' }]" @click="panel = panel === 'export' ? 'none' : 'export'"><IconDownload :size="18" stroke-width="1.75" /> Exportar</button>
          <button :class="['link', { on: panel === 'settings' }]" @click="panel = panel === 'settings' ? 'none' : 'settings'"><IconAdjustmentsHorizontal :size="18" stroke-width="1.75" /> Ajustes</button>
        </nav>
      </header>

      <p v-if="error" class="error banner" role="alert">{{ error }}</p>

      <Typewriter
        v-if="scene"
        :key="`${scene.id}-${editorKey}`"
        :scene-id="scene.id"
        :content-json="scene.content_json"
        :focus-mode="prefs.focus === 'on'"
        :key-sound="prefs.keysound === 'on'"
        @change="onChange"
      />
      <p v-else-if="outline" class="empty">Crie uma cena no sumário para começar.</p>

      <SettingsPanel v-if="panel === 'settings'" @close="panel = 'none'" />

      <aside v-if="panel === 'export'" class="versions" aria-label="Exportar o livro">
        <p class="hint">O livro inteiro, na ordem do sumário. Os títulos das cenas não saem no arquivo.</p>
        <div class="actions">
          <button class="btn" @click="exportBook('docx')">Exportar para Word</button>
          <button class="btn quiet" @click="exportBook('md')">Markdown</button>
        </div>
        <p v-if="exportMsg" class="hint export-msg">{{ exportMsg }}</p>
      </aside>

      <aside v-if="panel === 'versions'" class="versions" aria-label="Versões da cena">
        <p v-if="!versions.length" class="hint">Ainda não há versões salvas desta cena.</p>
        <ol v-else>
          <li v-for="v in versions" :key="v.id">
            <span>{{ when(v.created_at) }}</span>
            <span class="hint">{{ v.word_count.toLocaleString("pt-BR") }} palavras</span>
            <button class="link" @click="restore(v)">Restaurar</button>
          </li>
        </ol>
        <p class="hint">Restaurar guarda o texto atual como uma versão antes.</p>
      </aside>
    </main>
  </div>
</template>

<style scoped>
.editor { display: grid; grid-template-columns: 17rem 1fr; height: 100%; }
.editor.no-sidebar { grid-template-columns: 1fr; }
.desk { position: relative; display: flex; flex-direction: column; min-width: 0; height: 100%; }
.bar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  font-size: 0.85rem;
  border-bottom: 1px solid var(--rule);
}
.bar .icon { padding: 0 0.5rem; }
.tools { display: flex; gap: 0.15rem; }
.link.on { background: color-mix(in srgb, var(--ink) 9%, transparent); color: var(--ink); }
.scene-title {
  flex: 1;
  min-width: 0;
  padding: 0.35rem 0.6rem;
  border: 1px solid transparent;
  border-radius: 8px;
  background: none;
  font: 400 1rem var(--font-mono);
  color: var(--ink);
}
.scene-title:hover { border-color: var(--rule); }
.scene-title:focus { outline: none; border-color: var(--ink); }
.status { display: inline-flex; align-items: center; gap: 0.4rem; color: var(--note); font-size: 0.78rem; padding: 0 0.5rem; }
.dot { width: 6px; height: 6px; border-radius: 50%; background: #3c8a5f; }
.status.salvando .dot { background: #c99a2e; }
.status.erro { color: var(--ribbon); }
.status.erro .dot { background: var(--ribbon); }
.banner { padding: 0.5rem 1.25rem; margin: 0; }
.desk > :deep(.typewriter) { flex: 1; min-height: 0; }
.empty { color: var(--note); text-align: center; margin-top: 30vh; }
.versions {
  position: absolute;
  top: 3.4rem;
  right: 1rem;
  width: 20rem;
  max-height: 60vh;
  overflow-y: auto;
  padding: 1rem 1.25rem;
  background: var(--paper);
  border: 1px solid var(--rule);
  border-radius: 12px;
  box-shadow: 0 18px 40px -16px rgba(28, 35, 51, 0.35);
  z-index: 10;
  font-size: 0.85rem;
}
.versions ol { list-style: none; padding: 0; margin: 0 0 0.75rem; }
.versions li { display: grid; grid-template-columns: 1fr auto; gap: 0.1rem 1rem; padding: 0.5rem 0; border-bottom: 1px solid var(--rule); }
.versions li .link { grid-column: 2; grid-row: 1 / span 2; align-self: center; }
.export-msg { margin-top: 0.75rem; word-break: break-all; }
.hint { color: var(--note); font-size: 0.8rem; margin: 0; }
</style>
