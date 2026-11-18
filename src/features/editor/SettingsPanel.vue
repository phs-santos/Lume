<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { aiKeySet, aiKeyStatus, aiModelsGet, aiModelsSet, aiUsage } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import type { UsageRow } from "../../lib/types";
import { prefs, setPref } from "./settings";

const emit = defineEmits<{ close: [] }>();

const MODELS = ["anthropic/claude-sonnet-5-5", "anthropic/claude-haiku-5-5", "anthropic/claude-opus-5-5", "openai/gpt-5.4-mini", "deepseek/deepseek-v4-flash"];
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

/** "50 mil tokens", "1,2 mi tokens". */
/** A transcrição guarda milissegundos de áudio no lugar dos tokens: fica fora da soma. */
const STT_MODEL = "inworld/inworld-stt-1";
const llmUsage = computed(() => usage.value.filter((u) => u.model !== STT_MODEL));
const sttMinutes = computed(() =>
  Math.ceil(usage.value.filter((u) => u.model === STT_MODEL).reduce((t, u) => t + u.input_tokens, 0) / 60_000),
);

const usageText = computed(() => {
  const n = llmUsage.value.reduce((t, u) => t + u.input_tokens + u.output_tokens, 0);
  const cost = llmUsage.value.reduce((t, u) => t + (u.cost ?? 0), 0);
  const tokens = n < 1000 ? `${n} tokens` : n < 1e6 ? `${Math.round(n / 1000)} mil tokens` : `${(n / 1e6).toLocaleString("pt-BR", { maximumFractionDigits: 1 })} mi tokens`;
  return cost > 0 ? `${tokens} · ~US$ ${cost.toFixed(2)}` : tokens;
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
      <span>Esmaecer o resto do texto</span>
      <input type="checkbox" :checked="prefs.focus === 'on'" @change="setPref('focus', prefs.focus === 'on' ? 'off' : 'on')" />
    </label>
    <label class="row">
      <span>Som de tecla</span>
      <input type="checkbox" :checked="prefs.keysound === 'on'" @change="setPref('keysound', prefs.keysound === 'on' ? 'off' : 'on')" />
    </label>
    <section class="ai" aria-label="Assistente de IA">
      <h3>Assistente de IA</h3>
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
        <input v-model="chatModel" list="ai-models" @change="saveModels" />
      </label>
      <label class="field">
        <span>Modelo de fundo (resumos e fichas)</span>
        <input v-model="bgModel" list="ai-models" @change="saveModels" />
      </label>
      <datalist id="ai-models"><option v-for="m in MODELS" :key="m" :value="m" /></datalist>
      <p class="usage">Este mês: {{ usageText }}</p>
      <p v-if="sttMinutes > 0" class="usage">{{ sttMinutes }} min de transcrição este mês</p>
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
}
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
