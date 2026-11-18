<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { IconX } from "@tabler/icons-vue";
import { aiEnable, aiFirstPass, aiFirstPassCancel, aiKeyStatus, aiPendingCount } from "../../lib/api";
import type { Proposal } from "../../lib/types";
import BibleView from "./BibleView.vue";
import ChatView from "./ChatView.vue";

const props = defineProps<{ bookId: string; sceneId: string | null; mode: string; sceneTitles: Record<string, string>; draft?: string }>();
const emit = defineEmits<{
  close: [];
  openScene: [id: string];
  insertProposal: [p: Proposal];
  proposalAsScene: [p: Proposal];
  discardProposal: [p: Proposal];
}>();

const tab = ref<"chat" | "bible">("chat");
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

onMounted(async () => {
  try {
    await aiEnable(props.bookId);
    await firstPass();
  } catch {
    /* sem chave ou sem rede: a conversa mostra o erro quando a pessoa usar */
  }
});

onBeforeUnmount(() => {
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
        <button role="tab" :aria-selected="tab === 'chat'" :title="mode === 'coautora' ? 'Modo Coautora' : 'Modo Conselheira'" @click="tab = 'chat'">Conversa</button>
        <button role="tab" :aria-selected="tab === 'bible'" @click="tab = 'bible'">Bíblia</button>
      </div>
      <span class="spacer" />
      <button class="close" title="Fechar o assistente" aria-label="Fechar o assistente" @click="emit('close')"><IconX :size="16" stroke-width="1.75" /></button>
    </header>

    <div v-if="reading" class="reading" role="status">
      <span>Lendo o livro… {{ reading.done }}/{{ reading.total || "…" }} cenas</span>
      <div class="bar"><div :style="{ width: reading.total ? `${(reading.done / reading.total) * 100}%` : '0%' }" /></div>
      <button class="link" @click="cancelReading">Parar</button>
    </div>

    <ChatView
      v-show="tab === 'chat'"
      :book-id="bookId"
      :scene-id="sceneId"
      :mode="mode"
      :draft="draft"
      @insert-proposal="emit('insertProposal', $event)"
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
.close { display: inline-flex; padding: 0.3rem; border: 0; border-radius: 6px; background: none; color: #8b919c; cursor: pointer; }
.close:hover { color: #fff; background: rgba(255, 255, 255, 0.07); }
.reading { display: grid; grid-template-columns: 1fr auto; gap: 0.3rem 0.6rem; align-items: center; margin: 0 0.9rem 0.5rem; padding: 0.5rem 0.6rem; border-radius: 8px; background: rgba(255, 255, 255, 0.04); font-size: 0.78rem; }
.reading .bar { grid-column: 1; height: 3px; border-radius: 2px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }
.reading .bar div { height: 100%; background: var(--ribbon); transition: width 0.3s; }
.reading .link { grid-column: 2; grid-row: 1 / span 2; color: #aeb3bc; font-size: 0.78rem; }
</style>
