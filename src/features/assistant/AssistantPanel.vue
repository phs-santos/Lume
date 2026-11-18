<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { IconChevronDown, IconX } from "@tabler/icons-vue";
import { onClickOutside } from "../../lib/clickOutside";
import { aiEnable, aiFirstPass, aiFirstPassCancel, aiKeyStatus, aiPendingCount, settingGet, settingSet } from "../../lib/api";
import type { AiMode, Proposal } from "../../lib/types";
import BibleView from "./BibleView.vue";
import ChatView from "./ChatView.vue";

const props = defineProps<{ bookId: string; sceneId: string | null; mode: string; sceneTitles: Record<string, string>; draft?: string }>();
const emit = defineEmits<{
  close: [];
  openScene: [id: string];
  insertProposal: [p: Proposal, where: "end" | "cursor"];
  proposalAsScene: [p: Proposal];
  discardProposal: [p: Proposal];
  draftUsed: [];
  setMode: [m: AiMode];
}>();

const MODES: { id: AiMode; name: string; desc: string }[] = [
  { id: "conselheira", name: "Conselheira", desc: "Lê, opina e dá ideias. Nunca propõe texto." },
  { id: "coautora", name: "Coautora", desc: "Também propõe trechos, que só entram se você aprovar." },
];
const modeMenu = ref(false);
const modeBox = ref<HTMLElement | null>(null);
const stopModeOutside = onClickOutside(() => modeMenu.value, () => [modeBox.value], () => (modeMenu.value = false));
const modeName = () => MODES.find((m) => m.id === props.mode)?.name ?? props.mode;
function chooseMode(m: AiMode) {
  modeMenu.value = false;
  emit("setMode", m);
}

const chat = ref<{ focusInput: () => void; newConversation: () => Promise<void> } | null>(null);
/** Usado pelo ⌘J: mostra a conversa e põe o cursor no campo de mensagem. */
function focusInput() {
  tab.value = "chat";
  void nextTick(() => chat.value?.focusInput());
}
function newConversation() {
  tab.value = "chat";
  void nextTick(async () => {
    await chat.value?.newConversation();
    chat.value?.focusInput();
  });
}
const toggleTab = () => (tab.value = tab.value === "chat" ? "bible" : "chat");
defineExpose({ focusInput, newConversation, toggleTab });

const tab = ref<"chat" | "bible">("chat");
// uma ideia mandada para a conversa abre a aba Conversa, mesmo se a Bíblia estava aberta
watch(
  () => props.draft,
  (d) => {
    if (d) tab.value = "chat";
  },
  { immediate: true },
);
const reading = ref<{ done: number; total: number } | null>(null);

/** Primeira passada: lê só as cenas que ainda não têm resumo (ou mudaram). */
async function firstPass() {
  if (!(await aiKeyStatus()) || (await aiPendingCount(props.bookId)) === 0) return;
  reading.value = { done: 0, total: 0 };
  try {
    await aiFirstPass(props.bookId, (p) => (reading.value = p));
  } finally {
    reading.value = null;
  }
}

/**
 * A IA só lê o livro (resumos e Bíblia) com permissão, uma vez por livro.
 * Sem resposta ainda: o processamento de fundo fica pausado e o painel pergunta.
 */
const consentKey = () => `ai.consent.${props.bookId}`;
const asking = ref<number | null>(null); // nº de cenas a ler, enquanto a pergunta está na tela
const declined = ref(false);

async function startReading() {
  asking.value = null;
  declined.value = false;
  await settingSet(consentKey(), "sim");
  await aiEnable(props.bookId);
  await firstPass();
}

async function notNow() {
  asking.value = null;
  declined.value = true;
  await settingSet(consentKey(), "nao");
  await settingSet(`ai.enabled.${props.bookId}`, "0");
}

onMounted(async () => {
  try {
    if (!(await aiKeyStatus())) return;
    const consent = await settingGet(consentKey());
    if (consent === "sim") {
      await aiEnable(props.bookId);
      await firstPass();
    } else if (consent === "nao") {
      declined.value = true;
    } else {
      await settingSet(`ai.enabled.${props.bookId}`, "0"); // nada roda em segundo plano até ela responder
      const pending = await aiPendingCount(props.bookId);
      if (pending > 0) asking.value = pending;
      else await startReading();
    }
  } catch {
    /* sem chave ou sem rede: a conversa mostra o erro quando a pessoa usar */
  }
});

onBeforeUnmount(() => {
  stopModeOutside();
  if (reading.value) void aiFirstPassCancel();
});

function cancelReading() {
  void aiFirstPassCancel();
  reading.value = null;
}
</script>

<template>
  <aside class="assistant" aria-label="Assistente de IA">
    <header>
      <div class="tabs" role="tablist">
        <button role="tab" :aria-selected="tab === 'chat'" @click="tab = 'chat'">Conversa</button>
        <button role="tab" :aria-selected="tab === 'bible'" @click="tab = 'bible'">Bíblia</button>
      </div>
      <div ref="modeBox" class="mode">
        <button class="mode-btn" data-test="mode" :aria-expanded="modeMenu" title="Modo da IA neste livro" @click="modeMenu = !modeMenu">
          {{ modeName() }} <IconChevronDown :size="12" stroke-width="2.25" />
        </button>
        <div v-if="modeMenu" class="mode-menu" role="menu">
          <button v-for="m in MODES" :key="m.id" role="menuitemradio" :aria-checked="m.id === mode" @click="chooseMode(m.id)">
            <strong>{{ m.name }}</strong><small>{{ m.desc }}</small>
          </button>
        </div>
      </div>
      <span class="spacer" />
      <button class="close" title="Fechar o assistente" aria-label="Fechar o assistente" @click="emit('close')"><IconX :size="16" stroke-width="1.75" /></button>
    </header>

    <div v-if="asking" class="ask-read" role="region" aria-label="Ler o livro">
      <p><strong>Quer que eu leia o livro?</strong> São {{ asking }} cenas. Eu monto os resumos e a Bíblia (personagens, lugares, fatos) para conversar sabendo da história. Usa a sua chave da Inworld.</p>
      <div class="ask-actions">
        <button class="read-yes" data-test="read-yes" @click="startReading">Ler o livro</button>
        <button class="read-no" data-test="read-no" @click="notNow">Agora não</button>
      </div>
    </div>
    <p v-else-if="declined && tab === 'bible' && !reading" class="ask-read slim">
      A Bíblia não está sendo montada neste livro.
      <button class="read-yes" data-test="read-later" @click="startReading">Ler o livro agora</button>
    </p>

    <div v-if="reading" class="reading" role="status">
      <span>Lendo o livro… {{ reading.done }}/{{ reading.total || "…" }} cenas</span>
      <div class="bar"><div :style="{ width: reading.total ? `${(reading.done / reading.total) * 100}%` : '0%' }" /></div>
      <button class="link" @click="cancelReading">Parar</button>
    </div>

    <ChatView
      v-show="tab === 'chat'"
      ref="chat"
      :book-id="bookId"
      :scene-id="sceneId"
      :mode="mode"
      :scene-titles="sceneTitles"
      :draft="draft"
      @draft-used="emit('draftUsed')"
      @insert-proposal="(p, where) => emit('insertProposal', p, where)"
      @proposal-as-scene="emit('proposalAsScene', $event)"
      @discard-proposal="emit('discardProposal', $event)"
    />
    <BibleView v-if="tab === 'bible'" :book-id="bookId" :scene-titles="sceneTitles" @open-scene="emit('openScene', $event)" />
  </aside>
</template>

<style scoped>
.assistant { position: relative; display: flex; flex-direction: column; min-height: 0; min-width: 0; background: var(--platen); color: var(--platen-ink); border-left: 1px solid rgba(255, 255, 255, 0.06); }
header { display: flex; align-items: center; gap: 0.5rem; padding: 0.6rem 0.6rem 0.5rem 0.9rem; }
.tabs { display: flex; padding: 2px; border-radius: 8px; background: rgba(255, 255, 255, 0.05); }
.tabs button { border: 0; border-radius: 6px; padding: 0.25rem 0.7rem; background: none; color: #8b919c; font-size: 0.82rem; cursor: pointer; }
.tabs button[aria-selected="true"] { background: rgba(255, 255, 255, 0.1); color: #fff; }
:root[data-theme="escuro"] .tabs button[aria-selected="true"] { color: var(--ink); }
.spacer { flex: 1; }
.mode { position: relative; }
.mode-btn { display: inline-flex; align-items: center; gap: 0.2rem; padding: 0.2rem 0.5rem; border: 1px solid rgba(255, 255, 255, 0.12); border-radius: 999px; background: none; color: #c9cdd4; font-size: 0.74rem; cursor: pointer; }
.mode-btn:hover { color: #fff; border-color: rgba(255, 255, 255, 0.25); }
/* ancorado pela direita: o botão fica perto da borda do painel */
.mode-menu { position: absolute; top: calc(100% + 4px); right: 0; z-index: 20; width: 15rem; padding: 0.3rem; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 10px; background: #262b36; box-shadow: 0 16px 32px -12px rgba(0, 0, 0, 0.6); }
.mode-menu button { display: flex; flex-direction: column; gap: 0.1rem; width: 100%; padding: 0.45rem 0.55rem; border: 0; border-radius: 6px; background: none; color: #e6e8ec; text-align: left; cursor: pointer; }
.mode-menu button:hover { background: rgba(255, 255, 255, 0.07); }
.mode-menu button[aria-checked="true"] strong::after { content: " ✓"; color: var(--ribbon); }
.mode-menu strong { font-size: 0.82rem; }
.mode-menu small { font-size: 0.72rem; color: #8b919c; line-height: 1.35; }
.ask-read { margin: 0.75rem 0.75rem 0; padding: 0.8rem 0.9rem; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 10px; background: rgba(255, 255, 255, 0.04); color: #c9ccd3; font-size: 0.82rem; line-height: 1.45; }
.ask-read p { margin: 0 0 0.6rem; }
.ask-read strong { color: #fff; font-weight: 500; }
.ask-read.slim { display: flex; flex-wrap: wrap; align-items: center; gap: 0.5rem; }
.ask-actions { display: flex; gap: 0.4rem; }
.read-yes { padding: 0.35rem 0.75rem; border: 0; border-radius: 7px; background: var(--ribbon); color: #fff; cursor: pointer; font-size: 0.8rem; }
.read-no { padding: 0.35rem 0.75rem; border: 1px solid rgba(255, 255, 255, 0.15); border-radius: 7px; background: none; color: #c9ccd3; cursor: pointer; font-size: 0.8rem; }
.close { display: inline-flex; padding: 0.3rem; border: 0; border-radius: 6px; background: none; color: #8b919c; cursor: pointer; }
.close:hover { color: #fff; background: rgba(255, 255, 255, 0.07); }
.reading { display: grid; grid-template-columns: 1fr auto; gap: 0.3rem 0.6rem; align-items: center; margin: 0 0.9rem 0.5rem; padding: 0.5rem 0.6rem; border-radius: 8px; background: rgba(255, 255, 255, 0.04); font-size: 0.78rem; }
.reading .bar { grid-column: 1; height: 3px; border-radius: 2px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }
.reading .bar div { height: 100%; background: var(--ribbon); transition: width 0.3s; }
.reading .link { grid-column: 2; grid-row: 1 / span 2; color: #aeb3bc; font-size: 0.78rem; }
</style>
