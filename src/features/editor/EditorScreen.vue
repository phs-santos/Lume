<script setup lang="ts">
import { IconAdjustmentsHorizontal, IconArrowLeft, IconDownload, IconBulb, IconHistory, IconHelpCircle, IconLayoutSidebarLeftCollapse, IconLayoutSidebarLeftExpand, IconPhoto, IconSparkles } from "@tabler/icons-vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { toDocx } from "../../lib/export/docx";
import { toMarkdown } from "../../lib/export/markdown";
import { toPdf } from "../../lib/export/pdf";
import type { ExportBook } from "../../lib/export/types";
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { aiSceneTouched, bookSetMode, proposalSetStatus, bookOutline, chapterCreate, inboxList, inboxTranscribePending, remoteStatus, remoteSync, sceneCreate, sceneGet, sceneRestore, sceneSave, sceneVersions, settingGet, settingSet } from "../../lib/api";
import AssistantPanel from "../assistant/AssistantPanel.vue";
import { errorMessage } from "../../lib/errors";
import { countWords } from "../../lib/wordcount";
import type { AiMode, Outline as OutlineT, Proposal, Scene, Version } from "../../lib/types";
import Outline from "./Outline.vue";
import SettingsPanel from "./SettingsPanel.vue";
import Typewriter from "./Typewriter.vue";
import { loadPrefs, prefs, setPref } from "./settings";
import ShortcutsHelp from "./ShortcutsHelp.vue";
import { onClickOutside } from "../../lib/clickOutside";
import { imageAdd, imagesGc } from "../../lib/api";
import { prepareImage, toBase64 } from "./imageFile";
import BookSheet from "../book/BookSheet.vue";
import { coverColor } from "../../lib/coverColor";
import InboxPanel from "../inbox/InboxPanel.vue";
import { isMac, label, matchShortcut, SHORTCUTS, type Action } from "./shortcuts";
import { useAutosave } from "./useAutosave";
import { handleCloseRequest, registerFlusher } from "./exitGuard";

const props = defineProps<{ bookId: string }>();
const emit = defineEmits<{ close: [] }>();

const outline = ref<OutlineT | null>(null);
const scene = ref<Scene | null>(null);
const sidebar = ref(true);
const assistant = ref(false);

async function setAssistant(open: boolean) {
  assistant.value = open;
  try {
    await settingSet("assistant_open", open ? "1" : "0");
  } catch {
    /* preferência: não é grave se não salvar */
  }
}
const toggleAssistant = () => setAssistant(!assistant.value);

const assistantPanel = ref<{ focusInput: () => void; newConversation: () => void; toggleTab: () => void; $el: HTMLElement } | null>(null);

/** Atalhos do assistente: abrem o painel se estiver fechado. */
async function withAssistant(f: (p: NonNullable<typeof assistantPanel.value>) => void) {
  if (!assistant.value) await setAssistant(true);
  await nextTick();
  if (assistantPanel.value) f(assistantPanel.value);
}

/** ⌘J: abre o assistente e põe o cursor na mensagem; só fecha se o cursor já estiver nele. */
async function assistantShortcut() {
  const el = assistantPanel.value?.$el as HTMLElement | undefined;
  if (assistant.value && el?.contains(document.activeElement)) return setAssistant(false);
  if (!assistant.value) await setAssistant(true);
  await nextTick();
  assistantPanel.value?.focusInput();
}

/** Troca o modo da IA deste livro (Conselheira ou Coautora). */
async function setMode(m: AiMode) {
  if (!outline.value || outline.value.book.ai_mode === m) return;
  try {
    await bookSetMode(props.bookId, m);
    outline.value.book.ai_mode = m;
  } catch (e) {
    error.value = errorMessage(e);
  }
}

/** Avisa o agente de fundo que a cena mudou (ele ignora se o assistente não estiver ligado no livro). */
let idleTimer: ReturnType<typeof setTimeout> | null = null;
const touchScene = (id: string) => void aiSceneTouched(id).catch(() => {});
const panel = ref<"none" | "settings" | "versions" | "export" | "help" | "ideas">("none");
const bookSheet = ref(false); // ficha do livro (capa, autor, sinopse…)
const newIdeas = ref(0);
const mac = isMac();
const totalWords = computed(() =>
  (outline.value?.chapters ?? []).reduce((n, c) => n + c.scenes.reduce((m, x) => m + x.word_count, 0), 0),
);
const hint = (a: Action) => label(SHORTCUTS.find((s) => s.action === a)!, mac);
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
  // 2 minutos sem digitar: grava e atualiza resumo/Bíblia desta cena
  if (idleTimer) clearTimeout(idleTimer);
  idleTimer = setTimeout(async () => {
    if (await flushSafely()) touchScene(sceneId);
  }, 120_000);
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
  // a cena pode ter sido renomeada no sumário
  const t = outline.value.chapters.flatMap((c) => c.scenes).find((x) => x.id === scene.value?.id)?.title;
  if (scene.value && t) scene.value.title = t;
}

async function openScene(id: string) {
  if (scene.value?.id === id) return;
  if (!(await flushSafely())) return;
  const left = scene.value?.id;
  scene.value = await sceneGet(id);
  if (left) touchScene(left);
  panel.value = "none";
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

const exportScope = ref<"book" | "scene">("book");
const exportTitles = ref(false);
const FORMATS = {
  docx: { name: "Word", ext: "docx" },
  pdf: { name: "PDF", ext: "pdf" },
  md: { name: "Markdown", ext: "md" },
} as const;

async function exportBook(format: keyof typeof FORMATS) {
  exportMsg.value = "";
  if (!outline.value || !(await flushSafely())) return;
  try {
    const o = await bookOutline(props.bookId);
    const onlyScene = exportScope.value === "scene" ? scene.value?.id : null;
    const chapters = onlyScene
      ? o.chapters.filter((c) => c.scenes.some((s) => s.id === onlyScene)).map((c) => ({ ...c, scenes: c.scenes.filter((s) => s.id === onlyScene) }))
      : o.chapters;
    const book: ExportBook = {
      title: o.book.title,
      chapters: await Promise.all(
        chapters.map(async (c) => ({
          title: c.chapter.title,
          scenes: await Promise.all(
            c.scenes.map(async (s) => ({ title: s.title, doc: JSON.parse((await sceneGet(s.id)).content_json) })),
          ),
        })),
      ),
    };
    const base = onlyScene ? `${o.book.title} - ${scene.value?.title ?? "cena"}` : o.book.title;
    const name = base.replace(/[\\/:*?"<>|]/g, "-");
    const f = FORMATS[format];
    const path = await saveDialog({ defaultPath: `${name}.${f.ext}`, filters: [{ name: f.name, extensions: [f.ext] }] });
    if (!path) return;
    const opts = { sceneTitles: exportTitles.value };
    const bytes =
      format === "md" ? new TextEncoder().encode(toMarkdown(book, opts)) : format === "pdf" ? toPdf(book, opts) : await toDocx(book, opts);
    await writeFile(path, bytes);
    exportMsg.value = `Exportado para ${path}`;
  } catch (e) {
    exportMsg.value = `Não consegui exportar: ${errorMessage(e)}`;
  }
}

async function back() {
  if (await flushSafely()) emit("close");
}

/** Cenas do livro na ordem do sumário. */
const flatScenes = () => outline.value?.chapters.flatMap((c) => c.scenes) ?? [];

async function newSceneAfterCurrent() {
  const cur = flatScenes().find((x) => x.id === scene.value?.id);
  const ch = cur?.chapter_id ?? outline.value?.chapters[outline.value.chapters.length - 1]?.chapter.id;
  if (!ch) return;
  const count = outline.value!.chapters.find((c) => c.chapter.id === ch)!.scenes.length;
  const s = await sceneCreate(ch, `Cena ${count + 1}`, cur ? cur.position + 1 : undefined);
  await reloadOutline();
  await openScene(s.id);
}

async function newChapter() {
  if (!outline.value) return;
  const ch = await chapterCreate(props.bookId, `Capítulo ${outline.value.chapters.length + 1}`);
  const s = await sceneCreate(ch.id, "Cena 1");
  await reloadOutline();
  await openScene(s.id);
}

async function stepScene(delta: 1 | -1) {
  const list = flatScenes();
  const i = list.findIndex((x) => x.id === scene.value?.id);
  const next = list[i + delta];
  if (next) await openScene(next.id);
}

/** Conta as ideias novas para o selo do botão. */
async function refreshIdeas() {
  newIdeas.value = (await inboxList().catch(() => [])).filter((i) => i.status === "nova").length;
}

/** Busca no servidor em segundo plano, se este computador estiver conectado. */
async function backgroundSync() {
  try {
    if ((await remoteStatus()).connected) await remoteSync();
    await inboxTranscribePending();
  } catch {
    /* sem rede: tenta de novo no próximo ciclo */
  }
  await refreshIdeas();
}

const currentChapterTitle = computed(
  () => outline.value?.chapters.find((c) => c.chapter.id === scene.value?.chapter_id)?.chapter.title ?? "",
);

const typewriter = ref<{
  insertAtEnd: (t: string) => boolean;
  insertAtCursor: (t: string) => boolean;
  insertImage: (id: string, w: number, h: number) => boolean;
} | null>(null);

/** Imagens coladas, arrastadas ou escolhidas: reduz, guarda cifrada e insere no cursor. */
async function addImages(files: File[]) {
  if (!scene.value) return void (error.value = "Abra uma cena para inserir a imagem.");
  for (const f of files) {
    try {
      const img = await prepareImage(f);
      const id = await imageAdd(props.bookId, img.mime, toBase64(img.bytes), img.width, img.height);
      typewriter.value?.insertImage(id, img.width, img.height);
    } catch (e) {
      error.value = errorMessage(e);
    }
  }
}

const imagePicker = ref<HTMLInputElement | null>(null);
function pickImage() {
  imagePicker.value?.click();
}
async function onPicked(e: Event) {
  const input = e.target as HTMLInputElement;
  await addImages([...(input.files ?? [])]);
  input.value = "";
}

/** Ideia do celular entra no fim da cena aberta, pelo editor (um ⌘Z desfaz). Devolve se entrou. */
function insertIdea(text: string): boolean {
  if (!scene.value || !typewriter.value?.insertAtEnd(text)) {
    error.value = "Abra uma cena para inserir a ideia.";
    return false;
  }
  return true;
}

/** "Conversar sobre isso": abre o Assistente com a ideia já escrita no campo, sem enviar. */
const assistantDraft = ref("");
async function discussIdea(text: string) {
  panel.value = "none";
  if (!assistant.value) await toggleAssistant();
  assistantDraft.value = "";
  await nextTick();
  assistantDraft.value = text;
}

/** Proposta aprovada: entra pelo editor aberto (um ⌘Z desfaz), e o autosave grava. */
async function insertProposal(p: Proposal, where: "end" | "cursor" = "end") {
  try {
    if (p.scene_id && scene.value?.id !== p.scene_id) {
      await openScene(p.scene_id);
      await nextTick();
      await new Promise((r) => setTimeout(r, 50)); // o editor da cena nova precisa existir
    }
    // se a troca de cena não aconteceu (ex.: falha ao salvar), não insere na cena errada
    if (p.scene_id && scene.value?.id !== p.scene_id) throw new Error("Não consegui abrir a cena da proposta.");
    const tw = typewriter.value;
    const ok = where === "cursor" ? tw?.insertAtCursor(p.text) : tw?.insertAtEnd(p.text);
    if (!ok) throw new Error("Abra uma cena para inserir o trecho.");
    await proposalSetStatus(p.id, "inserida");
  } catch (e) {
    error.value = errorMessage(e);
  }
}

/** Proposta vira uma cena nova logo depois da cena de onde veio. */
async function proposalAsScene(p: Proposal) {
  try {
    if (!(await flushSafely()) || !outline.value) return;
    const ref = p.scene_id ?? scene.value?.id ?? null;
    const ch = outline.value.chapters.find((c) => c.scenes.some((s) => s.id === ref)) ?? outline.value.chapters[outline.value.chapters.length - 1];
    if (!ch) throw new Error("Crie um capítulo antes.");
    const index = ch.scenes.findIndex((s) => s.id === ref);
    const paragraphs = p.text.split(/\n\s*\n/).map((t) => t.trim()).filter(Boolean);
    const s = await sceneCreate(ch.chapter.id, paragraphs[0]?.slice(0, 40) || "Proposta", index >= 0 ? index + 1 : undefined);
    const doc = { type: "doc", content: paragraphs.map((t) => ({ type: "paragraph", content: [{ type: "text", text: t }] })) };
    await sceneSave(s.id, JSON.stringify(doc), countWords(paragraphs.join("\n")));
    await reloadOutline();
    await openScene(s.id);
    await proposalSetStatus(p.id, "virou_cena");
  } catch (e) {
    error.value = errorMessage(e);
  }
}

const discardProposal = (p: Proposal) => void proposalSetStatus(p.id, "descartada").catch((e) => (error.value = errorMessage(e)));

const sceneTitles = computed(() => Object.fromEntries(flatScenes().map((s) => [s.id, s.title])));

const currentSummary = computed(() => flatScenes().find((x) => x.id === scene.value?.id) ?? null);

async function onIdeaScene(id: string) {
  await reloadOutline();
  await openScene(id);
}

const togglePanel = (p: "settings" | "versions" | "export" | "help" | "ideas") =>
  p === "versions" ? showVersions() : (panel.value = panel.value === p ? "none" : p);

async function run(action: Action) {
  try {
    switch (action) {
      case "save": await flushSafely(); break;
      case "newScene": await newSceneAfterCurrent(); break;
      case "newChapter": await newChapter(); break;
      case "nextScene": await stepScene(1); break;
      case "prevScene": await stepScene(-1); break;
      case "toggleSidebar": sidebar.value = !sidebar.value; break;
      case "toggleFocus": await setPref("focus", prefs.focus === "on" ? "off" : "on"); break;
      case "toggleTheme": await setPref("theme", prefs.theme === "escuro" ? "papel" : "escuro"); break;
      case "versions": await togglePanel("versions"); break;
      case "export": togglePanel("export"); break;
      case "settings": togglePanel("settings"); break;
      case "help": togglePanel("help"); break;
      case "ideas": togglePanel("ideas"); break;
      case "assistant": await assistantShortcut(); break;
      case "newConversation": await withAssistant((p) => p.newConversation()); break;
      case "assistantTab": await withAssistant((p) => p.toggleTab()); break;
      case "close": panel.value = "none"; break;
    }
  } catch (e) {
    error.value = errorMessage(e);
  }
}

function onKey(e: KeyboardEvent) {
  const action = matchShortcut(e, mac);
  if (!action) return;
  if (action === "close" && panel.value === "none") return; // Esc sem painel: deixa para o editor
  e.preventDefault();
  void run(action);
}

const when = (t: number) =>
  new Date(t).toLocaleString("pt-BR", { day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit" });

let unlistenClose: (() => void) | null = null;
/** Painéis que abrem por cima da folha: fecham com clique fora ou Esc. */
const POPOVERS = ["ideas", "versions", "export", "settings"];
const stopClickOutside = onClickOutside(
  () => POPOVERS.includes(panel.value),
  () => [...document.querySelectorAll("[data-popover]"), document.querySelector("nav.tools")],
  () => (panel.value = "none"),
);
let syncTimer: ReturnType<typeof setInterval> | null = null;
const unregisterFlusher = registerFlusher(flushSafely);
const flushOnBlur = () => void flushSafely();

onMounted(async () => {
  void imagesGc(props.bookId).catch(() => {}); // tira do banco imagens que nenhuma cena ou versão usa
  window.addEventListener("keydown", onKey);
  window.addEventListener("blur", flushOnBlur);
  try {
    await loadPrefs();
    assistant.value = (await settingGet("assistant_open")) === "1";
    // primeira vez num livro: mostra os primeiros passos
    if ((await settingGet("onboarding_done")) !== "1") {
      panel.value = "help";
      void settingSet("onboarding_done", "1").catch(() => {});
    }
    await reloadOutline();
    const first = outline.value?.chapters.flatMap((c) => c.scenes)[0];
    if (first) scene.value = await sceneGet(first.id);
  } catch (e) {
    error.value = errorMessage(e);
  }
  void backgroundSync();
  syncTimer = setInterval(backgroundSync, 60_000);
  // Antes de a janela fechar, grava o pendente; se não conseguir, a janela não fecha.
  unlistenClose = await getCurrentWindow().onCloseRequested(handleCloseRequest);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKey);
  window.removeEventListener("blur", flushOnBlur);
  unlistenClose?.();
  unregisterFlusher();
  stopClickOutside();
  if (syncTimer) clearInterval(syncTimer);
  if (idleTimer) clearTimeout(idleTimer);
  if (scene.value) touchScene(scene.value.id);
});
</script>

<template>
  <div :class="['editor', { 'no-sidebar': !sidebar, 'with-assistant': assistant && outline }]">
      <header class="bar">
        <div v-if="sidebar && outline" class="book">
          <span class="book-title">{{ outline.book.title }}</span>
          <span class="book-words">{{ totalWords.toLocaleString("pt-BR") }} palavras</span>
        </div>
        <div class="bar-main">
        <button class="link icon" title="Voltar à biblioteca" aria-label="Voltar à biblioteca" @click="back"><IconArrowLeft :size="18" stroke-width="1.75" /></button>
        <button class="link icon" :title="`${sidebar ? 'Esconder' : 'Mostrar'} sumário (${hint('toggleSidebar')})`" :aria-label="sidebar ? 'Esconder sumário' : 'Mostrar sumário'" @click="sidebar = !sidebar">
          <IconLayoutSidebarLeftCollapse v-if="sidebar" :size="18" stroke-width="1.75" />
          <IconLayoutSidebarLeftExpand v-else :size="18" stroke-width="1.75" />
        </button>
        <span class="spacer" />
        <span class="status" :class="saveState" role="status">
          <span class="dot" />{{ saveState === "erro" ? "Não salvo" : saveState === "salvando" ? "Salvando…" : "Salvo" }}
        </span>
        <nav class="tools" aria-label="Ferramentas">
          <button :class="['link', 'icon', { on: panel === 'ideas' }]" :title="`Ideias que chegaram do celular (${hint('ideas')})`" aria-label="Ideias" @click="togglePanel('ideas')">
            <IconBulb :size="18" stroke-width="1.75" /><span class="lbl">Ideias</span><span v-if="newIdeas" class="badge">{{ newIdeas }}</span>
          </button>
          <button class="link icon" title="Inserir imagem no texto (ou cole/arraste na folha)" aria-label="Imagem" @click="pickImage"><IconPhoto :size="18" stroke-width="1.75" /><span class="lbl">Imagem</span></button>
          <input ref="imagePicker" type="file" accept="image/*" multiple hidden @change="onPicked" />
          <button :class="['link', 'icon', { on: panel === 'versions' }]" :title="`Versões salvas desta cena (${hint('versions')})`" aria-label="Versões" @click="showVersions"><IconHistory :size="18" stroke-width="1.75" /><span class="lbl">Versões</span></button>
          <button :class="['link', 'icon', { on: panel === 'export' }]" :title="`Exportar (${hint('export')})`" aria-label="Exportar" @click="panel = panel === 'export' ? 'none' : 'export'"><IconDownload :size="18" stroke-width="1.75" /><span class="lbl">Exportar</span></button>
          <button :class="['link', 'icon', { on: panel === 'settings' }]" :title="`Ajustes (${hint('settings')})`" aria-label="Ajustes" @click="panel = panel === 'settings' ? 'none' : 'settings'"><IconAdjustmentsHorizontal :size="18" stroke-width="1.75" /><span class="lbl">Ajustes</span></button>
          <button :class="['link', 'icon', { on: panel === 'help' }]" :title="`Ajuda e atalhos (${hint('help')})`" aria-label="Ajuda" @click="panel = panel === 'help' ? 'none' : 'help'"><IconHelpCircle :size="18" stroke-width="1.75" /><span class="lbl">Ajuda</span></button>
          <button :class="['link', 'icon', 'ai', { on: assistant }]" :title="`Assistente de IA (${hint('assistant')})`" aria-label="Assistente de IA" @click="toggleAssistant"><IconSparkles :size="18" stroke-width="1.75" /><span class="lbl">Assistente</span></button>
        </nav>
        </div>
      </header>

    <Outline
      v-if="outline && sidebar"
      :outline="outline"
      :current-scene-id="scene?.id ?? ''"
      @select="openScene"
      @changed="reloadOutline"
    />

    <main class="desk">
      <p v-if="error" class="error banner" role="alert">{{ error }}</p>

      <Typewriter
        v-if="scene"
        ref="typewriter"
        :key="`${scene.id}-${editorKey}`"
        :scene-id="scene.id"
        :content-json="scene.content_json"
        :focus-mode="prefs.focus === 'on'"
        :key-sound="prefs.keysound === 'on'"
        :chapter-title="currentChapterTitle"
        :scene-title="scene.title"
        :words="scene.word_count"
        @change="onChange"
        @image-files="addImages"
      />
      <p v-else-if="outline" class="empty">Crie uma cena no sumário para começar.</p>

      <SettingsPanel v-if="panel === 'settings'" :mode="outline?.book.ai_mode ?? null" @set-mode="setMode" @close="panel = 'none'" />
      <BookSheet v-if="bookSheet && outline" :book-id="bookId" :title="outline.book.title" :color="coverColor(bookId)" @close="bookSheet = false" />
      <ShortcutsHelp v-if="panel === 'help'" @close="panel = 'none'" />
      <InboxPanel
        v-if="panel === 'ideas'"
        :current="currentSummary"
        :fallback-chapter-id="outline?.chapters[0]?.chapter.id ?? null"
        @close="panel = 'none'"
        @scene-created="onIdeaScene"
        @changed="refreshIdeas"
        :insert-into-scene="insertIdea"
        @discuss="discussIdea"
      />

      <aside v-if="panel === 'export'" class="versions" data-popover aria-label="Exportar o livro">
        <div class="seg export-scope" role="radiogroup" aria-label="O que exportar">
          <button role="radio" :aria-checked="exportScope === 'book'" @click="exportScope = 'book'">Livro inteiro</button>
          <button role="radio" :aria-checked="exportScope === 'scene'" :disabled="!scene" @click="exportScope = 'scene'">Só esta cena</button>
        </div>
        <label class="check-row"><input v-model="exportTitles" type="checkbox" /> Incluir o título de cada cena</label>
        <p class="hint">Na ordem do sumário. Entre as cenas sai um separador (* * *).</p>
        <button class="link sheet-link" @click="(bookSheet = true), (panel = 'none')">Ficha do livro: capa, autor, sinopse e dedicatória</button>
        <div class="actions">
          <button class="btn" @click="exportBook('docx')">Word</button>
          <button class="btn quiet" @click="exportBook('pdf')">PDF</button>
          <button class="btn quiet" @click="exportBook('md')">Markdown</button>
        </div>
        <p v-if="exportMsg" class="hint export-msg">{{ exportMsg }}</p>
      </aside>

      <aside v-if="panel === 'versions'" class="versions" data-popover aria-label="Versões da cena">
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

    <AssistantPanel
      v-if="assistant && outline"
      ref="assistantPanel"
      :book-id="bookId"
      :scene-id="scene?.id ?? null"
      :mode="outline.book.ai_mode"
      :scene-titles="sceneTitles"
      :draft="assistantDraft"
      @draft-used="assistantDraft = ''"
      @close="toggleAssistant"
      @set-mode="setMode"
      @open-scene="openScene"
      @insert-proposal="insertProposal"
      @proposal-as-scene="proposalAsScene"
      @discard-proposal="discardProposal"
    />
  </div>
</template>

<style scoped>
/* Moldura escura em L: barra do topo + sumário formam uma peça só, a mesa fica dentro. */
.editor {
  display: grid;
  grid-template-columns: 17rem minmax(0, 1fr);
  grid-template-rows: auto minmax(0, 1fr);
  height: 100%;
  overflow: hidden;
  background: var(--platen);
}
.editor.no-sidebar { grid-template-columns: minmax(0, 1fr); }
/* assistente: terceira coluna à direita; a folha encolhe, nada fica coberto */
.editor.with-assistant { grid-template-columns: 17rem minmax(0, 1fr) 22rem; }
.editor.no-sidebar.with-assistant { grid-template-columns: minmax(0, 1fr) 22rem; }
.with-assistant .desk { border-top-right-radius: 14px; }
.desk {
  position: relative;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  border-top-left-radius: 14px;
  box-shadow: inset 0 2px 8px rgba(0, 0, 0, 0.35);
}
.no-sidebar .desk { border-radius: 0; }
.bar {
  grid-column: 1 / -1;
  display: grid;
  grid-template-columns: 17rem minmax(0, 1fr);
  align-items: center;
  min-height: 3.25rem;
  color: var(--platen-ink);
  font-size: 0.85rem;
}
.no-sidebar .bar { grid-template-columns: minmax(0, 1fr); }
.book { display: flex; flex-direction: column; padding: 0.4rem 1rem 0.4rem 1.25rem; min-width: 0; }
.book-title { font: 400 1rem/1.2 var(--font-mono); color: #fff; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.book-words { font-size: 0.72rem; color: #8b919c; }
.bar-main { display: flex; align-items: center; gap: 0.4rem; padding: 0.4rem 0.75rem; min-width: 0; }
.bar .link { color: #aeb3bc; }
.bar .link:hover, .bar .link.on { background: rgba(255, 255, 255, 0.08); color: #fff; }
.bar .icon { padding: 0 0.5rem; }
.tools { display: flex; gap: 0.15rem; }
.bar-main { container-type: inline-size; }
.bar .icon { display: inline-flex; align-items: center; gap: 0.35rem; }
.lbl { font-size: 0.8rem; }
/* pouco espaço: só os ícones (o nome continua na dica) */
@container (max-width: 44rem) { .lbl { display: none; } }
.spacer { flex: 1; }
.badge {
  min-width: 1.15rem;
  height: 1.15rem;
  padding: 0 0.3rem;
  border-radius: 999px;
  background: var(--ribbon);
  color: #fff;
  font-size: 0.7rem;
  line-height: 1.15rem;
  text-align: center;
}
.status { display: inline-flex; align-items: center; gap: 0.4rem; color: #8b919c; font-size: 0.78rem; padding: 0 0.5rem; }
.dot { width: 6px; height: 6px; border-radius: 50%; background: #3c8a5f; }
.status.salvando .dot { background: #c99a2e; }
.status.erro { color: #ff8a80; }
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
.sheet-link { margin: 0.25rem 0 0.5rem -0.5rem; font-size: 0.85rem; }
.export-msg { margin-top: 0.75rem; word-break: break-all; }
.export-scope { display: flex; margin-bottom: 0.6rem; padding: 2px; border: 1px solid var(--rule); border-radius: 8px; }
.export-scope button { flex: 1; border: 0; border-radius: 6px; background: none; padding: 0.3rem 0.5rem; color: var(--note); font-size: 0.82rem; cursor: pointer; }
.export-scope button[aria-checked="true"] { background: var(--ink); color: var(--paper); }
.check-row { display: flex; align-items: center; gap: 0.45rem; margin-bottom: 0.5rem; font-size: 0.85rem; }
.check-row input { accent-color: var(--ribbon); }
.versions .actions { display: flex; gap: 0.4rem; margin-top: 0.75rem; }
.hint { color: var(--note); font-size: 0.8rem; margin: 0; }
</style>
