<script setup lang="ts">
import { IconBookUpload, IconLock, IconPlus, IconX } from "@tabler/icons-vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { computed, nextTick, onMounted, ref } from "vue";
import DeskScene from "../../components/DeskScene.vue";
import { aiKeyStatus, bookCreate, bookImportEpub, bookList, vaultLock } from "../../lib/api";
import { coverColor } from "../../lib/coverColor";
import { errorMessage } from "../../lib/errors";
import { greeting, timeOfDay } from "../../lib/timeOfDay";
import type { AiMode, Book } from "../../lib/types";

const emit = defineEmits<{ open: [bookId: string, opts?: { idea?: string }]; locked: [] }>();
const books = ref<Book[]>([]);
const creating = ref(false);
const title = ref("");
const mode = ref<AiMode>("conselheira");
/** "escrever": abre o livro na folha; "ideia": abre no mapa, conversando com a IA sobre a ideia. */
const start = ref<"escrever" | "ideia">("escrever");
const idea = ref("");
const ideaBox = ref<HTMLTextAreaElement | null>(null);
const hasKey = ref(false);
const canCreate = computed(() => start.value === "escrever" || !!idea.value.trim());
const error = ref("");
const titleInput = ref<HTMLInputElement | null>(null);
const forced = import.meta.env.DEV ? new URLSearchParams(location.search).get("hora") : null;
const time = computed(() => timeOfDay(forced ? Number(forced) : new Date().getHours()));

const modes: { value: AiMode; name: string; hint: string }[] = [
  { value: "conselheira", name: "Conselheira", hint: "A IA só sugere. Nunca escreve no seu texto." },
  { value: "coautora", name: "Coautora", hint: "A IA escreve rascunhos que você aprova." },
];

async function load() {
  try {
    books.value = await bookList();
  } catch (e) {
    error.value = errorMessage(e);
  }
}

async function startCreating() {
  title.value = "";
  mode.value = "conselheira";
  start.value = "escrever";
  idea.value = "";
  creating.value = true;
  aiKeyStatus().then((k) => (hasKey.value = k)).catch(() => (hasKey.value = false));
  await nextTick();
  titleInput.value?.focus();
}

async function chooseIdea() {
  start.value = "ideia";
  await nextTick();
  ideaBox.value?.focus();
}

async function create() {
  if (!canCreate.value) return;
  try {
    const b = await bookCreate(title.value.trim(), mode.value);
    emit("open", b.id, start.value === "ideia" ? { idea: idea.value.trim() } : undefined);
  } catch (e) {
    error.value = errorMessage(e);
  }
}

const importing = ref(false);

async function importEpub() {
  error.value = "";
  const path = await openDialog({ multiple: false, filters: [{ name: "Livro EPUB", extensions: ["epub"] }] });
  if (typeof path !== "string") return;
  importing.value = true;
  try {
    const b = await bookImportEpub(path, "conselheira");
    emit("open", b.id);
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    importing.value = false;
  }
}

async function lock() {
  await vaultLock();
  emit("locked");
}

const year = (t: number) => new Date(t).getFullYear();

onMounted(load);
</script>

<template>
  <div class="library">
    <header class="hero">
      <DeskScene :time="time" crop="middle" class="scene" />
      <button class="link lock" @click="lock"><IconLock :size="18" stroke-width="1.75" /> Trancar</button>
    </header>

    <main class="shelf">
      <div class="shelf-head">
        <h1>Seus livros</h1>
        <p>{{ greeting[time] }}</p>
      </div>
      <p v-if="error" class="error">{{ error }}</p>

      <ul class="covers">
        <li v-for="b in books" :key="b.id">
          <button class="cover" :style="{ '--cover': coverColor(b.id) }" @click="emit('open', b.id)">
            <span class="cover-title">{{ b.title }}</span>
            <span v-if="b.title_provisional" class="cover-tag">nome provisório</span>
            <span class="cover-foot">
              <span>{{ b.ai_mode === "coautora" ? "Coautora" : "Conselheira" }}</span>
              <span>{{ year(b.created_at) }}</span>
            </span>
          </button>
        </li>
        <li>
          <button class="cover new" @click="startCreating">
            <IconPlus :size="28" stroke-width="1.5" />
            <span>{{ books.length ? "Novo livro" : "Comece seu primeiro livro" }}</span>
          </button>
        </li>
      </ul>
      <p class="import">
        Já tem um livro pronto em outro programa?
        <button class="link" :disabled="importing" @click="importEpub">
          <IconBookUpload :size="16" stroke-width="1.75" /> {{ importing ? "Importando…" : "Trazer de um arquivo EPUB" }}
        </button>
        <small>EPUB é o formato de livro digital que o Word, o Google Docs e o Scrivener conseguem gerar.</small>
      </p>
    </main>

    <div v-if="creating" class="backdrop" @click.self="creating = false">
      <form class="dialog" role="dialog" aria-labelledby="new-book-title" @submit.prevent="create" @keydown.esc="creating = false">
        <div class="dialog-head">
          <h2 id="new-book-title">Novo livro</h2>
          <button type="button" class="link icon" aria-label="Fechar" @click="creating = false"><IconX :size="18" /></button>
        </div>
        <label class="field name">
          <span>Nome do livro</span>
          <input ref="titleInput" v-model="title" placeholder="Livro sem nome" />
          <small>Pode ser um nome provisório. Dá para trocar depois.</small>
        </label>
        <fieldset class="modes">
          <legend>Como você quer começar?</legend>
          <label :class="['mode-opt', { on: start === 'escrever' }]">
            <input v-model="start" type="radio" name="start" value="escrever" />
            <span><strong>Escrever direto</strong><small>Abre a primeira cena, pronta para escrever.</small></span>
          </label>
          <label :class="['mode-opt', { on: start === 'ideia', off: !hasKey }]">
            <input :checked="start === 'ideia'" type="radio" name="start" value="ideia" :disabled="!hasKey" @change="chooseIdea" />
            <span>
              <strong>Tenho uma ideia e quero desenvolver com a IA</strong>
              <small v-if="hasKey">A IA conversa com você e monta um mapa de ideias.</small>
              <small v-else>Para usar, configure a IA em Ajustes do assistente.</small>
            </span>
          </label>
          <textarea
            v-if="start === 'ideia'"
            ref="ideaBox"
            v-model="idea"
            class="idea"
            rows="3"
            aria-label="A ideia do livro"
            placeholder="Conte a ideia do jeito que vier…"
            @keydown.enter.meta.prevent="create"
            @keydown.enter.ctrl.prevent="create"
          />
        </fieldset>
        <fieldset class="modes ai">
          <legend>Como a IA vai ajudar neste livro</legend>
          <label v-for="m in modes" :key="m.value" :class="['mode-opt', { on: mode === m.value }]">
            <input v-model="mode" type="radio" name="mode" :value="m.value" />
            <span><strong>{{ m.name }}</strong><small>{{ m.hint }}</small></span>
          </label>
        </fieldset>
        <p class="mode-later">Dá para trocar depois, no painel do assistente ou em Ajustes.</p>
        <button class="btn wide" :disabled="!canCreate">Criar livro</button>
      </form>
    </div>
  </div>
</template>

<style scoped>
.library { height: 100%; overflow-y: auto; scrollbar-width: thin; }

.hero { position: relative; height: clamp(12rem, 34vh, 20rem); overflow: hidden; }
.scene { position: absolute; inset: 0; }
.shelf-head { margin-bottom: 2rem; }
.shelf-head h1 { font: 400 2.4rem/1.1 var(--font-mono); margin: 0; letter-spacing: -0.02em; }
.shelf-head p { font: italic 1.05rem var(--font-serif); color: var(--note); margin: 0.35rem 0 0; }
.lock { position: absolute; top: 1rem; right: 1rem; z-index: 1; background: color-mix(in srgb, var(--paper) 85%, transparent); color: var(--ink); }

.shelf { padding: 2rem clamp(1.5rem, 6vw, 5rem) 4rem; }
.import { margin: 2.5rem 0 0; font-size: 0.88rem; color: var(--note); }
.import .link { display: inline-flex; align-items: center; gap: 0.3rem; }
.import small { display: block; margin-top: 0.25rem; font-size: 0.78rem; }
.mode-later { margin: -0.25rem 0 1rem; font-size: 0.8rem; color: var(--note); }
.covers {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(9.5rem, 1fr));
  gap: 2rem 1.75rem;
}
.cover {
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  width: 100%;
  aspect-ratio: 2 / 3;
  padding: 1.25rem 1rem 0.9rem 1.5rem;
  border: 0;
  border-radius: 3px 8px 8px 3px;
  background: var(--cover);
  color: #f3f1ea;
  text-align: left;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2), 0 12px 24px -12px rgba(10, 14, 30, 0.5);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}
/* lombada: faixa mais escura e um vinco */
.cover::before {
  content: "";
  position: absolute;
  inset: 0 auto 0 0;
  width: 0.7rem;
  border-radius: 3px 0 0 3px;
  background: linear-gradient(to right, rgba(0, 0, 0, 0.35), rgba(0, 0, 0, 0.05));
}
.cover:hover { transform: translateY(-6px) rotate(-0.6deg); box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2), 0 22px 30px -14px rgba(10, 14, 30, 0.55); }
.cover-title { font: 500 1.2rem/1.25 var(--font-serif); overflow-wrap: anywhere; }
.cover-foot { display: flex; justify-content: space-between; font-size: 0.75rem; opacity: 0.75; }

.cover.new {
  align-items: center;
  justify-content: center;
  gap: 0.6rem;
  padding: 1rem;
  background: transparent;
  color: var(--note);
  border: 1.5px dashed var(--rule);
  border-radius: 8px;
  box-shadow: none;
  text-align: center;
}
.cover.new::before { display: none; }
.cover.new:hover { color: var(--ink); border-color: var(--ink); transform: none; box-shadow: none; }

.backdrop {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 1rem;
  background: rgba(15, 20, 38, 0.45);
  z-index: 20;
}
.dialog {
  width: min(30rem, 100%);
  max-height: calc(100vh - 2rem);
  overflow-y: auto;
  padding: 1.5rem 1.75rem 1.75rem;
  background: var(--paper);
  border-radius: 14px;
  box-shadow: 0 30px 60px -20px rgba(10, 14, 30, 0.5);
}
.dialog-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
.dialog-head h2 { margin: 0; font: 500 1.2rem var(--font-ui); }
.icon { padding: 0 0.45rem; }
.modes { border: 0; padding: 0; margin: 0 0 1.25rem; display: grid; gap: 0.6rem; }
.modes legend { color: var(--note); font-size: 0.85rem; margin-bottom: 0.5rem; }
.mode-opt {
  display: flex;
  gap: 0.75rem;
  align-items: flex-start;
  padding: 0.8rem 0.9rem;
  border: 1px solid var(--rule);
  border-radius: 10px;
  cursor: pointer;
  transition: border-color 0.15s, background-color 0.15s;
}
.mode-opt.on { border-color: var(--ink); background: color-mix(in srgb, var(--ink) 4%, transparent); }
.mode-opt input { accent-color: var(--ribbon); margin-top: 0.25rem; }
.mode-opt strong { display: block; font-weight: 500; }
.mode-opt small { color: var(--note); font-size: 0.82rem; }
.mode-opt.off { cursor: default; opacity: 0.6; }
.modes.ai { grid-template-columns: 1fr 1fr; }
.modes.ai legend { grid-column: 1 / -1; }
.modes.ai .mode-opt { padding: 0.6rem 0.7rem; gap: 0.5rem; }
.name small { display: block; margin-top: 0.35rem; color: var(--note); font-size: 0.8rem; }
.name input::placeholder { color: color-mix(in srgb, var(--note) 70%, transparent); }
.idea {
  width: 100%;
  padding: 0.6rem 0.8rem;
  border: 1px solid var(--rule);
  border-radius: 8px;
  background: color-mix(in srgb, #fff 70%, var(--paper));
  font: 0.95rem/1.5 var(--font-serif);
  resize: vertical;
}
.idea:focus { outline: 2px solid var(--ribbon); outline-offset: 1px; }
.cover-tag { align-self: flex-start; margin: 0.5rem 0 auto; padding: 0.05rem 0.5rem; border: 1px solid currentColor; border-radius: 999px; font: 0.7rem/1.5 var(--font-ui); opacity: 0.7; }
</style>
