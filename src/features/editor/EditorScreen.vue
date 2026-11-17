<script setup lang="ts">
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

const autosave = useAutosave(async (json, words) => {
  const s = scene.value;
  if (!s) return;
  saveState.value = "salvando";
  try {
    await sceneSave(s.id, json, words);
    s.content_json = json;
    s.word_count = words;
    for (const c of outline.value?.chapters ?? [])
      for (const x of c.scenes) if (x.id === s.id) x.word_count = words;
    saveState.value = "salvo";
  } catch (e) {
    saveState.value = "erro";
    throw e;
  }
});

/** Só aceita alterações do editor da cena atual: um editor antigo nunca grava na cena nova. */
function onChange(sceneId: string, json: string, words: number) {
  if (sceneId !== scene.value?.id) return;
  autosave.schedule(json, words);
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

onMounted(async () => {
  window.addEventListener("keydown", onKey);
  try {
    await loadPrefs();
    await reloadOutline();
    const first = outline.value?.chapters.flatMap((c) => c.scenes)[0];
    if (first) scene.value = await sceneGet(first.id);
  } catch (e) {
    error.value = errorMessage(e);
  }
  // Antes de a janela fechar, grava o que estiver pendente.
  unlistenClose = await getCurrentWindow().onCloseRequested(async () => {
    await flushSafely();
  });
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKey);
  unlistenClose?.();
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
        <button class="link" @click="back">Livros</button>
        <input
          v-if="scene"
          class="scene-title"
          :value="scene.title"
          aria-label="Título da cena"
          @blur="renameScene"
          @keydown.enter="($event.target as HTMLInputElement).blur()"
        />
        <span class="status" :class="saveState">{{ saveState === "erro" ? "Não salvo" : saveState === "salvando" ? "Salvando…" : "Salvo" }}</span>
        <button class="link" @click="showVersions">Versões</button>
        <button class="link" @click="panel = panel === 'export' ? 'none' : 'export'">Exportar</button>
        <button class="link" @click="panel = panel === 'settings' ? 'none' : 'settings'">Ajustes</button>
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
  gap: 1.25rem;
  padding: 0.75rem 1.25rem;
  font-size: 0.85rem;
  border-bottom: 1px solid var(--rule);
}
.bar .link { text-decoration: none; }
.bar .link:hover { color: var(--ink); }
.scene-title {
  flex: 1;
  min-width: 0;
  border: 0;
  background: none;
  font: 400 1rem var(--font-mono);
  color: var(--ink);
}
.scene-title:focus { outline: none; border-bottom: 1px solid var(--rule); }
.status { color: var(--note); font-size: 0.78rem; }
.status.erro { color: var(--ribbon); }
.banner { padding: 0.5rem 1.25rem; margin: 0; }
.desk > :deep(.typewriter) { flex: 1; min-height: 0; }
.empty { color: var(--note); text-align: center; margin-top: 30vh; }
.versions {
  position: absolute;
  top: 3rem;
  right: 1rem;
  width: 20rem;
  max-height: 60vh;
  overflow-y: auto;
  padding: 1rem 1.25rem;
  background: var(--paper);
  border: 1px solid var(--rule);
  box-shadow: 0 8px 24px rgba(28, 35, 51, 0.12);
  z-index: 10;
  font-size: 0.85rem;
}
.versions ol { list-style: none; padding: 0; margin: 0 0 0.75rem; }
.versions li { display: grid; grid-template-columns: 1fr auto; gap: 0.1rem 1rem; padding: 0.5rem 0; border-bottom: 1px solid var(--rule); }
.versions li .link { grid-column: 2; grid-row: 1; }
.export-msg { margin-top: 0.75rem; word-break: break-all; }
.hint { color: var(--note); font-size: 0.8rem; margin: 0; }
</style>
