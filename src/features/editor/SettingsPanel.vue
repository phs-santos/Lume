<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { aiKeySet, aiKeyStatus, aiModelsGet, aiModelsSet, aiUsage, settingGet, settingSet } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import type { AiMode, UsageRow } from "../../lib/types";
import { prefs, setPref } from "./settings";

const props = withDefaults(defineProps<{ mode?: AiMode | null }>(), { mode: null });
const emit = defineEmits<{ close: []; setMode: [m: AiMode] }>();

/** Modelos sugeridos, com o preço (US$ por milhão de tokens de entrada / saída). */
const MODELS: { id: string; name: string; price: string }[] = [
  { id: "anthropic/claude-haiku-5-5", name: "Claude Haiku 5.5 · rápido e barato", price: "US$ 0,10 / 0,50" },
  { id: "anthropic/claude-sonnet-5-5", name: "Claude Sonnet 5.5 · equilibrado", price: "US$ 2 / 10" },
  { id: "anthropic/claude-opus-5-5", name: "Claude Opus 5.5 · o mais capaz", price: "US$ 4 / 20" },
];
const OTHER = "__outro";
const choice = (id: string) => (MODELS.some((m) => m.id === id) ? id : OTHER);
const limit = ref("");
const hasKey = ref(false);
const editingKey = ref(false);
const keyDraft = ref("");
const chatModel = ref("");
const bgModel = ref("");
const usage = ref<UsageRow[]>([]);
const aiError = ref("");

const month = () => new Date().toISOString().slice(0, 7);

onMounted(async () => {
  try {
    hasKey.value = await aiKeyStatus();
    const m = await aiModelsGet();
    chatModel.value = m.chat;
    bgModel.value = m.background;
    usage.value = await aiUsage(month());
    limit.value = (await settingGet("ai.monthly_limit")) ?? "";
  } catch (e) {
    aiError.value = errorMessage(e);
  }
});

async function saveKey() {
  aiError.value = "";
  try {
    await aiKeySet(keyDraft.value);
    keyDraft.value = "";
    hasKey.value = true;
    editingKey.value = false;
  } catch (e) {
    aiError.value = errorMessage(e);
  }
}

async function saveModels() {
  if (!chatModel.value.trim() || !bgModel.value.trim()) return;
  try {
    await aiModelsSet(chatModel.value, bgModel.value);
  } catch (e) {
    aiError.value = errorMessage(e);
  }
}

async function saveLimit() {
  const v = limit.value.replace(",", ".").trim();
  if (v && !(Number(v) > 0)) return void (aiError.value = "O limite precisa ser um valor em dólares, como 5 ou 2,50.");
  aiError.value = "";
  try {
    await settingSet("ai.monthly_limit", v);
  } catch (e) {
    aiError.value = errorMessage(e);
  }
}

function pickModel(which: "chat" | "bg", id: string) {
  const target = which === "chat" ? chatModel : bgModel;
  target.value = id === OTHER ? "" : id;
  if (id !== OTHER) void saveModels();
}

/** "50 mil tokens", "1,2 mi tokens". */
const usageText = computed(() => {
  const n = usage.value.reduce((t, u) => t + u.input_tokens + u.output_tokens, 0);
  const cost = usage.value.reduce((t, u) => t + (u.cost ?? 0), 0);
  const tokens = n < 1000 ? `${n} tokens` : n < 1e6 ? `${Math.round(n / 1000)} mil tokens` : `${(n / 1e6).toLocaleString("pt-BR", { maximumFractionDigits: 1 })} mi tokens`;
  return cost > 0 ? `${tokens} · ~US$ ${cost.toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}` : tokens;
});
</script>

<template>
  <aside class="panel" aria-label="Ajustes">
    <div class="row">
      <span>Letra</span>
      <div class="seg">
        <button :aria-pressed="prefs.font === 'serif'" class="serif" @click="setPref('font', 'serif')">Livro</button>
        <button :aria-pressed="prefs.font === 'mono'" class="mono" @click="setPref('font', 'mono')">Máquina</button>
      </div>
    </div>
    <div class="row">
      <span>Papel</span>
      <div class="seg">
        <button :aria-pressed="prefs.theme === 'papel'" @click="setPref('theme', 'papel')">Claro</button>
        <button :aria-pressed="prefs.theme === 'escuro'" @click="setPref('theme', 'escuro')">Escuro</button>
      </div>
    </div>
    <label class="row">
      <span>Modo foco <small>esmaece o resto do texto</small></span>
      <input type="checkbox" :checked="prefs.focus === 'on'" @change="setPref('focus', prefs.focus === 'on' ? 'off' : 'on')" />
    </label>
    <label class="row">
      <span>Som de tecla</span>
      <input type="checkbox" :checked="prefs.keysound === 'on'" @change="setPref('keysound', prefs.keysound === 'on' ? 'off' : 'on')" />
    </label>
    <section class="ai" aria-label="Assistente de IA">
      <h3>Assistente de IA</h3>
      <div v-if="props.mode" class="row">
        <span>Neste livro</span>
        <div class="seg">
          <button :aria-pressed="props.mode === 'conselheira'" title="Lê, opina e dá ideias. Nunca propõe texto." @click="emit('setMode', 'conselheira')">Conselheira</button>
          <button :aria-pressed="props.mode === 'coautora'" title="Também propõe trechos, que só entram se você aprovar." @click="emit('setMode', 'coautora')">Coautora</button>
        </div>
      </div>
      <div v-if="hasKey && !editingKey" class="row">
        <span>Chave salva</span>
        <button class="link" @click="editingKey = true">Trocar</button>
      </div>
      <form v-else class="key" @submit.prevent="saveKey">
        <input v-model="keyDraft" type="password" placeholder="Chave da Inworld" aria-label="Chave da Inworld" autocomplete="off" />
        <button class="btn" data-test="save-key" type="submit" :disabled="!keyDraft.trim()">Salvar</button>
      </form>
      <label class="field">
        <span>Modelo da conversa</span>
        <select data-test="chat-model" :value="choice(chatModel)" @change="pickModel('chat', ($event.target as HTMLSelectElement).value)">
          <option v-for="m in MODELS" :key="m.id" :value="m.id">{{ m.name }}</option>
          <option :value="OTHER">Outro (digitar o nome)</option>
        </select>
        <input v-if="choice(chatModel) === OTHER" v-model="chatModel" placeholder="provedor/modelo" aria-label="Nome do modelo da conversa" @change="saveModels" />
      </label>
      <label class="field">
        <span>Modelo de fundo (resumos e fichas)</span>
        <select data-test="bg-model" :value="choice(bgModel)" @change="pickModel('bg', ($event.target as HTMLSelectElement).value)">
          <option v-for="m in MODELS" :key="m.id" :value="m.id">{{ m.name }}</option>
          <option :value="OTHER">Outro (digitar o nome)</option>
        </select>
        <input v-if="choice(bgModel) === OTHER" v-model="bgModel" placeholder="provedor/modelo" aria-label="Nome do modelo de fundo" @change="saveModels" />
      </label>
      <p class="prices">Preço por milhão de tokens (entrada / saída): {{ MODELS.map((m) => `${m.name.split(" ·")[0].replace("Claude ", "")} ${m.price}`).join(" · ") }}. Outros modelos aparecem só em tokens.</p>
      <label class="field">
        <span>Limite de gasto por mês (US$)</span>
        <input v-model="limit" data-test="limit" inputmode="decimal" placeholder="sem limite" @change="saveLimit" />
      </label>
      <p class="usage">Este mês: {{ usageText }}</p>
      <p v-if="aiError" class="err" role="alert">{{ aiError }}</p>
    </section>
    <button class="link" @click="emit('close')">Fechar</button>
  </aside>
</template>

<style scoped>
.panel {
  position: absolute;
  top: 3.4rem;
  right: 1rem;
  width: 18rem;
  padding: 1rem 1.25rem;
  background: var(--paper);
  border: 1px solid var(--rule);
  border-radius: 12px;
  box-shadow: 0 18px 40px -16px rgba(28, 35, 51, 0.35);
  z-index: 10;
  font-size: 0.9rem;
  max-height: calc(100% - 4.5rem);
  overflow-y: auto;
  scrollbar-width: thin;
}
.row small { display: block; font-size: 0.74rem; color: var(--note); }
.field select { padding: 0.35rem 0.4rem; border: 1px solid var(--rule); border-radius: 6px; background: var(--sheet); font-size: 0.82rem; color: var(--ink); }
.prices { margin: 0.2rem 0 0.3rem; font-size: 0.72rem; line-height: 1.4; color: var(--note); }
.row { display: flex; justify-content: space-between; align-items: center; gap: 1rem; padding: 0.45rem 0; }
.row input { accent-color: var(--ribbon); width: 1rem; height: 1rem; }
.seg { display: flex; padding: 2px; border: 1px solid var(--rule); border-radius: 8px; }
.seg button { border: 0; border-radius: 6px; background: none; padding: 0.25rem 0.65rem; cursor: pointer; color: var(--note); }
.seg button[aria-pressed="true"] { background: var(--ink); color: var(--paper); }
.seg .serif { font-family: var(--font-serif); }
.seg .mono { font-family: var(--font-mono); }
.link { margin-top: 0.5rem; }
.ai { margin-top: 0.6rem; padding-top: 0.6rem; border-top: 1px solid var(--rule); }
.ai h3 { margin: 0 0 0.3rem; font-size: 0.8rem; font-weight: 600; color: var(--note); text-transform: uppercase; letter-spacing: 0.05em; }
.ai .row .link { margin: 0; }
.key { display: flex; gap: 0.4rem; padding: 0.35rem 0; }
.key input, .field input { flex: 1; min-width: 0; padding: 0.35rem 0.5rem; border: 1px solid var(--rule); border-radius: 6px; background: var(--sheet); font-size: 0.82rem; }
.field { display: flex; flex-direction: column; gap: 0.2rem; padding: 0.3rem 0; font-size: 0.82rem; color: var(--note); }
.usage { margin: 0.4rem 0 0; font-size: 0.8rem; color: var(--note); }
.err { margin: 0.3rem 0 0; font-size: 0.8rem; color: var(--ribbon); }
</style>
