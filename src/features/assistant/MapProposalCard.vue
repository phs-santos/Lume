<script setup lang="ts">
import { computed, ref } from "vue";
import { mapProposalAccept, mapProposalDiscard } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import type { MapCard, MapProposal } from "../../lib/types";

const props = defineProps<{ proposal: MapProposal }>();
const emit = defineEmits<{ accepted: [cards: MapCard[]]; renamed: [title: string]; discarded: [] }>();

interface Arrow { de: string; para: string; rotulo?: string; de_texto?: string; para_texto?: string }
interface Payload { cartoes?: { ref: string; texto: string }[]; setas?: Arrow[]; nome?: string }
const payload = computed<Payload>(() => {
  try {
    return JSON.parse(props.proposal.payload_json);
  } catch {
    return {};
  }
});
const cards = computed(() => payload.value.cartoes ?? []);
const chosen = ref<string[]>(cards.value.map((c) => c.ref));
const firstLine = (t: string) => t.trim().split("\n")[0].trim();
/** Setas que entram com os cartões marcados, com o título de cada ponta. */
const links = computed(() => {
  const name = (end: string, known?: string) => {
    const c = cards.value.find((k) => k.ref === end);
    if (c) return chosen.value.includes(end) ? firstLine(c.texto) : null;
    return known ?? "um cartão do mapa";
  };
  return (payload.value.setas ?? []).flatMap((a) => {
    const from = name(a.de, a.de_texto), to = name(a.para, a.para_texto);
    return from && to ? [`${from} → ${to}${a.rotulo ? ` (${a.rotulo})` : ""}`] : [];
  });
});
const busy = ref(false);
const error = ref("");

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
const title = computed(() => `A IA sugere ${plural(cards.value.length, "cartão", "cartões")} para o mapa`);

async function run(fn: () => Promise<void>) {
  busy.value = true;
  error.value = "";
  try {
    await fn();
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}

const accept = () => run(async () => {
  const made = await mapProposalAccept(props.proposal.id, props.proposal.kind === "nome" ? [] : chosen.value);
  if (props.proposal.kind === "nome") emit("renamed", payload.value.nome ?? "");
  else emit("accepted", made);
});
const discard = () => run(async () => {
  await mapProposalDiscard(props.proposal.id);
  emit("discarded");
});
</script>

<template>
  <div class="proposal map-proposal">
    <template v-if="proposal.kind === 'nome'">
      <p class="label">Sugestão de nome</p>
      <p class="question">Usar “{{ payload.nome }}” como nome do livro?</p>
      <div class="actions">
        <button class="btn small" :disabled="busy" @click="accept">Usar</button>
        <button class="link" :disabled="busy" @click="discard">Agora não</button>
      </div>
    </template>
    <template v-else>
      <p class="label">{{ title }}</p>
      <ul class="cards">
        <li v-for="c in cards" :key="c.ref">
          <label>
            <input v-model="chosen" type="checkbox" :value="c.ref" :disabled="busy" />
            <span class="card-text">{{ c.texto }}</span>
          </label>
        </li>
      </ul>
      <template v-if="links.length">
        <p class="label sub">Ligações</p>
        <ul class="arrows">
          <li v-for="(l, i) in links" :key="i">{{ l }}</li>
        </ul>
      </template>
      <div class="actions">
        <button class="btn small" :disabled="busy || !chosen.length" @click="accept">Pôr no mapa</button>
        <button class="link" :disabled="busy" @click="discard">Descartar</button>
      </div>
    </template>
    <p v-if="error" class="err" role="alert">{{ error }}</p>
  </div>
</template>

<style scoped>
.proposal { padding: 0.6rem 0.7rem; border: 1px solid rgba(255, 255, 255, 0.12); border-left: 2px solid var(--ribbon); border-radius: 0 8px 8px 0; background: rgba(255, 255, 255, 0.03); }
.label { margin: 0 0 0.4rem; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.05em; color: #8b919c; }
.question { margin: 0 0 0.6rem; font-family: var(--font-serif); font-size: 0.92rem; }
.cards { list-style: none; margin: 0 0 0.4rem; padding: 0; display: grid; gap: 0.3rem; }
.cards label { display: flex; align-items: flex-start; gap: 0.5rem; cursor: pointer; }
.cards input { margin-top: 0.25rem; accent-color: var(--ribbon); }
.card-text { white-space: pre-wrap; font-family: var(--font-serif); font-size: 0.88rem; line-height: 1.45; }
.label.sub { margin-top: 0.5rem; }
.arrows { list-style: none; margin: 0 0 0.6rem; padding: 0; display: grid; gap: 0.15rem; font-size: 0.82rem; color: #b9bdc4; }
.actions { display: flex; flex-wrap: wrap; align-items: center; gap: 0.4rem; }
.btn.small { padding: 0.3rem 0.6rem; font-size: 0.78rem; }
.err { margin: 0.5rem 0 0; color: #ff8a80; font-size: 0.8rem; }
</style>
