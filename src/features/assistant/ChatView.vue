<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from "vue";
import { IconArrowUp, IconMessages, IconPencil, IconPlus, IconRefresh, IconSearch, IconTrash } from "@tabler/icons-vue";
import {
  aiConversation, aiConversationDelete, aiConversationNew, aiConversationOpen, aiConversationRename, aiConversations, aiKeySet, aiKeyStatus, aiRetry, aiSend, proposalsPending,
} from "../../lib/api";
import { openUrl } from "@tauri-apps/plugin-opener";
import { errorMessage } from "../../lib/errors";
import type { ChatEvent, Conversation, Proposal } from "../../lib/types";

const props = defineProps<{ bookId: string; sceneId: string | null; mode: string; sceneTitles?: Record<string, string>; draft?: string }>();
const emit = defineEmits<{
  insertProposal: [p: Proposal, where: "end" | "cursor"];
  proposalAsScene: [p: Proposal];
  discardProposal: [p: Proposal];
  draftUsed: [];
}>();

/** Perguntas prontas para quem não sabe por onde começar. */
const SUGGESTIONS: Record<string, string[]> = {
  conselheira: ["O que você achou desta cena?", "Tem alguma contradição no livro até aqui?", "Resuma o livro até agora", "Quem são os personagens e o que cada um quer?"],
  coautora: ["Sugira como continuar esta cena", "Escreva um parágrafo de transição para a próxima cena", "O que você achou desta cena?", "Tem alguma contradição no livro até aqui?"],
};

type Item =
  | { kind: "user"; id: string; text: string }
  | { kind: "assistant"; id: string; text: string; meta?: string }
  | { kind: "note"; id: string; text: string }
  | { kind: "proposal"; id: string; proposal: Proposal };

const hasKey = ref<boolean | null>(null);
const keyDraft = ref("");
const items = ref<Item[]>([]);
const draft = ref("");
// rascunho vindo de fora (ex.: "Conversar sobre esta ideia"): preenche o campo, não envia,
// e avisa que usou, para não voltar quando o painel for reaberto
function takeDraft(d?: string) {
  if (!d) return;
  draft.value = d;
  emit("draftUsed");
}
watch(() => props.draft, takeDraft);
const busy = ref(false);
const error = ref("");
const showHistory = ref(false);
const history = ref<Conversation[]>([]);
const scroller = ref<HTMLElement | null>(null);
const input = ref<HTMLTextAreaElement | null>(null);
const renaming = ref<string | null>(null);
const renameDraft = ref("");
const confirmDel = ref<string | null>(null);

/** "5,2 mil tokens · ~US$ 0,01" */
function costLine(i: number, o: number, cost: number | null) {
  const n = i + o;
  const tokens = n < 1000 ? `${n} tokens` : `${(n / 1000).toLocaleString("pt-BR", { maximumFractionDigits: 1 })} mil tokens`;
  if (cost == null) return tokens;
  const usd = cost < 0.01 ? "menos de US$ 0,01" : `~US$ ${cost.toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  return `${tokens} · ${usd}`;
}
let seq = 0;
const tmpId = () => `tmp-${++seq}`;

function load(view: { messages: { id: string; role: string; content: string }[] }) {
  items.value = view.messages.map((m) => ({ kind: m.role, id: m.id, text: m.content }) as Item);
  error.value = "";
  void scrollDown();
}

async function scrollDown() {
  await nextTick();
  const el = scroller.value;
  if (el) el.scrollTop = el.scrollHeight;
}

onMounted(async () => {
  takeDraft(props.draft);
  try {
    hasKey.value = await aiKeyStatus();
    load(await aiConversation(props.bookId));
    // propostas ainda sem decisão continuam esperando, mesmo depois de fechar o app
    if (props.mode === "coautora")
      for (const p of await proposalsPending(props.bookId)) items.value.push({ kind: "proposal", id: p.id, proposal: p });
  } catch (e) {
    error.value = errorMessage(e);
  }
});

async function saveKey() {
  try {
    await aiKeySet(keyDraft.value);
    keyDraft.value = "";
    hasKey.value = true;
  } catch (e) {
    error.value = errorMessage(e);
  }
}

/** Recebe os eventos de uma resposta; a bolha da IA só é criada quando chega o primeiro pedaço. */
function handler() {
  let bubbleId: string | null = null;
  const mine: string[] = []; // bolhas criadas nesta resposta
  const bubble = () => items.value.find((i) => i.id === bubbleId) as { text: string } | undefined;
  return (e: ChatEvent) => {
    if (e.type === "delta") {
      if (!bubble()) {
        bubbleId = tmpId();
        mine.push(bubbleId);
        items.value.push({ kind: "assistant", id: bubbleId, text: "" });
      }
      bubble()!.text += e.text;
    } else if (e.type === "note") {
      items.value.push({ kind: "note", id: tmpId(), text: e.text });
      bubbleId = null; // texto depois da consulta vai numa bolha nova
    } else if (e.type === "proposal") {
      if (props.mode === "coautora") items.value.push({ kind: "proposal", id: e.proposal.id, proposal: e.proposal });
    } else if (e.type === "done") {
      // as bolhas desta resposta já estão salvas: não são mais "provisórias"
      const created = new Set(mine);
      items.value = items.value.map((i) => (created.has(i.id) ? { ...i, id: `${e.message_id}-${i.id}` } : i));
      // o gasto desta resposta aparece embaixo da última bolha dela
      const last = [...items.value].reverse().find((i) => i.kind === "assistant" && i.id.startsWith(`${e.message_id}-`));
      if (last && last.kind === "assistant") last.meta = costLine(e.input_tokens, e.output_tokens, e.cost);
      mine.length = 0;
    } else if (e.type === "error") {
      // descarta só a resposta pela metade desta chamada; a pergunta continua salva
      const created = new Set(mine);
      items.value = items.value.filter((i) => !(i.kind === "assistant" && created.has(i.id)));
      error.value = e.message;
    }
    void scrollDown();
  };
}

async function go(call: (on: (e: ChatEvent) => void) => Promise<void>) {
  busy.value = true;
  error.value = "";
  try {
    await call(handler());
  } catch (e) {
    if (!error.value) error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}

async function send(preset?: string) {
  if (preset) draft.value = preset;
  const text = draft.value.trim();
  if (!text || busy.value) return;
  draft.value = "";
  items.value.push({ kind: "user", id: tmpId(), text });
  void scrollDown();
  await go((on) => aiSend(props.bookId, props.sceneId, text, on));
}

const retry = () => go((on) => aiRetry(props.bookId, props.sceneId, on));

function onKey(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
    e.preventDefault();
    void send();
  }
}

async function newConversation() {
  showHistory.value = false;
  load(await aiConversationNew(props.bookId));
}

async function toggleHistory() {
  showHistory.value = !showHistory.value;
  if (showHistory.value) history.value = await aiConversations(props.bookId);
}

async function openConversation(id: string) {
  if (renaming.value) return;
  showHistory.value = false;
  load(await aiConversationOpen(id));
}

async function startRename(c: Conversation) {
  confirmDel.value = null;
  renaming.value = c.id;
  renameDraft.value = c.title;
  await nextTick();
  (document.getElementById(`conv-rename-${c.id}`) as HTMLInputElement | null)?.select();
}

async function finishRename(c: Conversation) {
  if (renaming.value !== c.id) return;
  renaming.value = null;
  const t = renameDraft.value.trim();
  if (!t || t === c.title) return;
  try {
    await aiConversationRename(c.id, t);
    c.title = t;
  } catch (e) {
    error.value = errorMessage(e);
  }
}

/** Apagar pede um segundo clique. Se era a conversa aberta, começa outra. */
async function removeConversation(c: Conversation) {
  if (confirmDel.value !== c.id) return void (confirmDel.value = c.id);
  confirmDel.value = null;
  try {
    await aiConversationDelete(c.id);
    history.value = history.value.filter((x) => x.id !== c.id);
    if (!c.closed) load(await aiConversation(props.bookId));
  } catch (e) {
    error.value = errorMessage(e);
  }
}

const targetName = (p: Proposal) => (p.scene_id ? props.sceneTitles?.[p.scene_id] : null) ?? "cena atual";

function resolveProposal(p: Proposal, action: "insert" | "cursor" | "scene" | "discard") {
  items.value = items.value.filter((i) => !(i.kind === "proposal" && i.id === p.id));
  if (action === "insert" || action === "cursor") emit("insertProposal", p, action === "insert" ? "end" : "cursor");
  else if (action === "scene") emit("proposalAsScene", p);
  else emit("discardProposal", p);
}

const KEY_HELP = "https://platform.inworld.ai/";
const openKeyHelp = () => void openUrl(KEY_HELP).catch(() => window.open(KEY_HELP, "_blank"));

defineExpose({ focusInput: () => input.value?.focus(), newConversation: () => (busy.value ? Promise.resolve() : newConversation()), addProposals: (ps: Proposal[]) => ps.forEach((p) => items.value.push({ kind: "proposal", id: p.id, proposal: p })) });
</script>

<template>
  <div class="chat">
    <form v-if="hasKey === false" class="need-key" @submit.prevent="saveKey">
      <p class="lead">O assistente é opcional: o Lume funciona inteiro sem ele.</p>
      <p>
        Para usar, ele precisa de uma chave da <strong>Inworld</strong>, o serviço que roda os modelos de IA.
        Crie uma conta, gere uma chave em “API Keys” e cole aqui. Você paga à Inworld só o que usar:
        uma conversa costuma custar centavos. Dá para pôr um limite de gasto em Ajustes.
      </p>
      <button type="button" class="link how" @click="openKeyHelp">Como conseguir a chave ↗</button>
      <input v-model="keyDraft" type="password" placeholder="Chave da Inworld" aria-label="Chave da Inworld" autocomplete="off" />
      <button class="btn" type="submit" :disabled="!keyDraft.trim()">Salvar chave</button>
      <p class="fine">A chave fica guardada cifrada neste computador.</p>
      <p v-if="error" class="err" role="alert">{{ error }}</p>
    </form>

    <template v-else-if="hasKey">
      <div class="tools">
        <button class="tool" data-test="new-conv" title="Nova conversa" aria-label="Nova conversa" :disabled="busy" @click="newConversation"><IconPlus :size="16" stroke-width="1.75" /></button>
        <button :class="['tool', { on: showHistory }]" data-test="history" title="Conversas anteriores" aria-label="Conversas anteriores" @click="toggleHistory"><IconMessages :size="16" stroke-width="1.75" /></button>
      </div>

      <ul v-if="showHistory" class="history" aria-label="Conversas anteriores">
        <li v-for="c in history" :key="c.id" class="conv-row">
          <input
            v-if="renaming === c.id"
            :id="`conv-rename-${c.id}`"
            v-model="renameDraft"
            class="conv-rename"
            aria-label="Novo nome da conversa"
            @keydown.enter.prevent="finishRename(c)"
            @keydown.esc="renaming = null"
            @blur="finishRename(c)"
          />
          <button v-else class="conv-item" :disabled="busy" @click="openConversation(c.id)">
            <span>{{ c.title }}</span>
            <small>{{ new Date(c.updated_at).toLocaleDateString("pt-BR", { day: "2-digit", month: "short" }) }}{{ c.closed ? "" : " · atual" }}</small>
          </button>
          <button class="tool" data-test="conv-rename" title="Renomear" aria-label="Renomear conversa" @click="startRename(c)"><IconPencil :size="13" stroke-width="2" /></button>
          <button :class="['tool', 'del', { confirm: confirmDel === c.id }]" data-test="conv-delete" :title="confirmDel === c.id ? 'Clique de novo para apagar' : 'Apagar'" aria-label="Apagar conversa" :disabled="busy" @click="removeConversation(c)">
            <IconTrash :size="13" stroke-width="2" /><span v-if="confirmDel === c.id">Apagar?</span>
          </button>
        </li>
        <li v-if="!history.length" class="empty">Nenhuma conversa ainda.</li>
      </ul>

      <div ref="scroller" class="log" aria-live="polite">
        <template v-if="!items.length && !busy">
          <p class="empty">
            {{ mode === "coautora" ? "Peça ideias, revisões ou um trecho: propostas só entram no livro se você aprovar." : "Pergunte sobre o livro, peça opinião ou ideias. A Conselheira não mexe no seu texto." }}
          </p>
          <div class="suggestions" aria-label="Sugestões de pergunta">
            <button v-for="q in SUGGESTIONS[mode] ?? SUGGESTIONS.conselheira" :key="q" class="chip" data-test="suggestion" @click="send(q)">{{ q }}</button>
          </div>
        </template>
        <template v-for="it in items" :key="it.id">
          <p v-if="it.kind === 'note'" class="note"><IconSearch :size="12" stroke-width="2" /> {{ it.text }}</p>
          <div v-else-if="it.kind === 'proposal'" class="proposal">
            <p class="proposal-label">Proposta de trecho · para “{{ targetName(it.proposal) }}”</p>
            <p class="proposal-text">{{ it.proposal.text }}</p>
            <div class="proposal-actions">
              <button class="btn small" data-test="p-insert" @click="resolveProposal(it.proposal, 'insert')">Inserir no fim da cena</button>
              <button class="btn small quiet" data-test="p-cursor" :disabled="!!it.proposal.scene_id && it.proposal.scene_id !== sceneId" title="Insere onde está o cursor (só na cena aberta)" @click="resolveProposal(it.proposal, 'cursor')">No cursor</button>
              <button class="btn small quiet" data-test="p-scene" @click="resolveProposal(it.proposal, 'scene')">Virar cena nova</button>
              <button class="link" data-test="p-discard" @click="resolveProposal(it.proposal, 'discard')">Descartar</button>
            </div>
          </div>
          <template v-else>
            <div :class="['msg', it.kind]">{{ it.text }}</div>
            <p v-if="it.kind === 'assistant' && it.meta" class="meta" data-test="meta">{{ it.meta }}</p>
          </template>
        </template>
        <p v-if="busy" class="typing" aria-label="A IA está respondendo"><span /><span /><span /></p>
        <p v-if="error" class="err" role="alert">
          {{ error }}
          <button class="link" data-test="retry" @click="retry"><IconRefresh :size="13" stroke-width="2" /> Tentar de novo</button>
        </p>
      </div>

      <form class="composer" @submit.prevent="send()">
        <textarea ref="input" v-model="draft" rows="2" placeholder="Pergunte algo…" title="Enter envia · Shift+Enter quebra linha" aria-label="Mensagem para a IA" @keydown="onKey" />
        <button class="send" type="submit" title="Enviar" aria-label="Enviar" :disabled="busy || !draft.trim()"><IconArrowUp :size="16" stroke-width="2.25" /></button>
      </form>
    </template>
  </div>
</template>

<style scoped>
.chat { display: flex; flex-direction: column; min-height: 0; height: 100%; }
/* fica no cabeçalho do painel, ao lado do X (o painel é position: relative) */
.tools { position: absolute; top: 0.6rem; right: 2.6rem; display: flex; gap: 0.15rem; }
.tool { display: inline-flex; padding: 0.3rem; border: 0; border-radius: 6px; background: none; color: #8b919c; cursor: pointer; }
.tool:hover, .tool.on { color: #fff; background: rgba(255, 255, 255, 0.07); }
.history { list-style: none; margin: 0 0.75rem 0.5rem; padding: 0.3rem; border-radius: 8px; background: rgba(255, 255, 255, 0.04); max-height: 40%; overflow-y: auto; }
.conv-item { display: flex; justify-content: space-between; gap: 0.5rem; width: 100%; padding: 0.4rem 0.5rem; border: 0; border-radius: 6px; background: none; color: inherit; text-align: left; cursor: pointer; font-size: 0.82rem; }
.conv-item span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.conv-item small { flex: none; color: #8b919c; }
.conv-item:hover { background: rgba(255, 255, 255, 0.06); }
.conv-row { display: flex; align-items: center; gap: 0.1rem; }
.conv-row .conv-item { flex: 1; min-width: 0; }
.conv-row .tool { opacity: 0; }
.conv-row:hover .tool, .conv-row .tool:focus-visible, .conv-row .tool.confirm { opacity: 1; }
.tool.del.confirm { gap: 0.25rem; color: #ff8a80; font-size: 0.72rem; }
.conv-rename { flex: 1; min-width: 0; margin: 0.15rem 0.2rem; padding: 0.3rem 0.45rem; border: 1px solid rgba(255, 255, 255, 0.2); border-radius: 6px; background: rgba(0, 0, 0, 0.2); color: inherit; font-size: 0.82rem; }
.suggestions { display: flex; flex-direction: column; align-items: flex-start; gap: 0.35rem; }
.chip { padding: 0.35rem 0.65rem; border: 1px solid rgba(255, 255, 255, 0.12); border-radius: 999px; background: none; color: #c9cdd4; font-size: 0.8rem; text-align: left; cursor: pointer; }
.chip:hover { color: #fff; border-color: rgba(255, 255, 255, 0.28); background: rgba(255, 255, 255, 0.04); }
.meta { margin: -0.3rem 0 0; font-size: 0.7rem; color: #737985; }
.need-key .lead { color: #fff; }
.need-key .how { align-self: flex-start; padding: 0; color: #fff; font-size: 0.82rem; text-decoration: underline; }
.need-key .fine { font-size: 0.74rem; color: #8b919c; }
.log { flex: 1; min-height: 0; overflow-y: auto; padding: 0.25rem 0.9rem 0.75rem; display: flex; flex-direction: column; gap: 0.55rem; scrollbar-width: thin; scrollbar-color: rgba(255, 255, 255, 0.15) transparent; }
.empty { color: #8b919c; font-size: 0.82rem; margin: 0.5rem 0; }
.msg { white-space: pre-wrap; overflow-wrap: anywhere; font-size: 0.88rem; line-height: 1.55; }
.msg.user { align-self: flex-end; max-width: 88%; padding: 0.45rem 0.7rem; border-radius: 12px 12px 4px 12px; background: rgba(255, 255, 255, 0.09); color: #fff; }
:root[data-theme="escuro"] .msg.user { color: var(--ink); }
.msg.assistant { color: var(--platen-ink); }
.note { display: flex; align-items: center; gap: 0.3rem; margin: 0; font-size: 0.74rem; font-style: italic; color: #8b919c; }
.proposal { padding: 0.6rem 0.7rem; border: 1px solid rgba(255, 255, 255, 0.12); border-left: 2px solid var(--ribbon); border-radius: 0 8px 8px 0; background: rgba(255, 255, 255, 0.03); }
.proposal-label { margin: 0 0 0.3rem; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.05em; color: #8b919c; }
.proposal-text { margin: 0 0 0.6rem; white-space: pre-wrap; font-family: var(--font-serif); font-size: 0.9rem; line-height: 1.6; }
.proposal-actions { display: flex; flex-wrap: wrap; align-items: center; gap: 0.4rem; }
.btn.small { padding: 0.3rem 0.6rem; font-size: 0.78rem; }
.typing { display: flex; gap: 0.25rem; margin: 0; }
.typing span { width: 5px; height: 5px; border-radius: 50%; background: #8b919c; animation: blink 1.2s infinite; }
.typing span:nth-child(2) { animation-delay: 0.2s; }
.typing span:nth-child(3) { animation-delay: 0.4s; }
@keyframes blink { 0%, 80%, 100% { opacity: 0.25; } 40% { opacity: 1; } }
.err { margin: 0; font-size: 0.8rem; color: #ff8a80; }
.err .link { display: inline-flex; align-items: center; gap: 0.25rem; margin-left: 0.4rem; color: #fff; }
.composer { position: relative; padding: 0.6rem 0.75rem 0.75rem; border-top: 1px solid rgba(255, 255, 255, 0.06); }
.composer textarea { width: 100%; resize: none; padding: 0.55rem 2.6rem 0.55rem 0.7rem; border: 1px solid rgba(255, 255, 255, 0.12); border-radius: 10px; background: rgba(0, 0, 0, 0.18); color: var(--platen-ink); font-size: 0.86rem; line-height: 1.45; }
.composer textarea:focus { outline: none; border-color: rgba(255, 255, 255, 0.3); }
.send { position: absolute; right: 1.2rem; bottom: 1.35rem; display: grid; place-items: center; width: 1.8rem; height: 1.8rem; border: 0; border-radius: 50%; background: var(--ribbon); color: #fff; cursor: pointer; }
.send:disabled { opacity: 0.35; cursor: default; }
.need-key { display: flex; flex-direction: column; gap: 0.6rem; padding: 0.5rem 0.9rem; font-size: 0.85rem; }
.need-key p { margin: 0; color: #aeb3bc; }
.need-key input { padding: 0.45rem 0.6rem; border: 1px solid rgba(255, 255, 255, 0.15); border-radius: 8px; background: rgba(0, 0, 0, 0.18); color: var(--platen-ink); }
</style>
