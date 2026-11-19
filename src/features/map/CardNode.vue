<script setup lang="ts">
import { computed, inject, nextTick, ref } from "vue";
import { Handle, Position, type NodeProps } from "@vue-flow/core";
import { IconDots } from "@tabler/icons-vue";
import { CARD_COLORS, type BibleEntry, type CardColor } from "../../lib/types";
import { MAP_ACTIONS, type MapActions } from "./actions";
import type { CardData } from "./flow";

const props = defineProps<NodeProps<CardData>>();
const actions = inject(MAP_ACTIONS) as MapActions;
const card = computed(() => props.data.card);

const COLOR_NAMES: Record<CardColor, string> = { papel: "Papel", amarelo: "Amarelo", verde: "Verde", azul: "Azul", rosa: "Rosa" };
const KIND_NAMES: Record<string, string> = { personagem: "Personagem", lugar: "Lugar", fato: "Fato", linha_do_tempo: "Linha do tempo" };

const editing = ref(false);
const draft = ref("");
const box = ref<HTMLTextAreaElement | null>(null);
const menu = ref(false);
const kind = computed(() => (card.value.bible_entry_id ? actions.kindOf(card.value.bible_entry_id) : null));

async function edit() {
  draft.value = card.value.text;
  editing.value = true;
  await nextTick();
  box.value?.focus();
  box.value?.select();
}
defineExpose({ edit });
if (actions.consumeEditRequest(props.id)) void edit();

async function save() {
  if (!editing.value) return;
  editing.value = false;
  if (draft.value !== card.value.text) await actions.save(props.id, draft.value, card.value.color);
}
function keydown(e: KeyboardEvent) {
  if (e.key === "Escape") { e.preventDefault(); void save(); }
  if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) { e.preventDefault(); void save(); }
}
async function pick(fn: () => Promise<unknown>) {
  menu.value = false;
  await fn();
}
const bibleKinds: BibleEntry["kind"][] = ["personagem", "lugar", "fato"];
</script>

<template>
  <div :class="['card', `c-${card.color}`, { editing }]" @dblclick.stop="edit">
    <Handle type="target" :position="Position.Top" class="dot" />
    <span v-if="kind" class="badge">{{ KIND_NAMES[kind] ?? kind }}</span>
    <button class="more nodrag" title="Opções do cartão" aria-label="Opções do cartão" @click.stop="menu = !menu" @dblclick.stop>
      <IconDots :size="15" stroke-width="2" />
    </button>
    <textarea
      v-if="editing"
      ref="box"
      v-model="draft"
      class="text nodrag nowheel"
      aria-label="Texto do cartão"
      rows="4"
      @blur="save"
      @keydown.stop="keydown"
    />
    <p v-else :class="['text', { empty: !card.text.trim() }]">{{ card.text.trim() || "Cartão vazio" }}</p>
    <div v-if="menu" class="menu nodrag" role="menu" @dblclick.stop>
      <div class="colors" role="group" aria-label="Cor">
        <button
          v-for="c in CARD_COLORS"
          :key="c"
          :class="['swatch', `c-${c}`, { on: c === card.color }]"
          :title="COLOR_NAMES[c]"
          :aria-label="`Cor ${COLOR_NAMES[c]}`"
          @click="pick(() => actions.save(id, card.text, c))"
        />
      </div>
      <button v-for="k in bibleKinds" :key="k" class="menu-item" role="menuitem" :disabled="!!card.bible_entry_id" @click="pick(() => actions.toBible(id, k))">
        Transformar em {{ KIND_NAMES[k].toLowerCase() }}
      </button>
      <button class="menu-item danger" role="menuitem" @click="pick(() => actions.remove(id))">Apagar cartão</button>
    </div>
    <Handle type="source" :position="Position.Bottom" class="dot" />
  </div>
</template>

<style scoped>
.card {
  position: relative;
  width: 240px;
  min-height: 72px;
  padding: 0.7rem 1.9rem 0.7rem 0.85rem;
  border: 1px solid color-mix(in srgb, var(--ink) 14%, transparent);
  border-radius: 10px;
  background: var(--map-papel);
  color: var(--ink);
  box-shadow: 0 6px 16px -10px rgba(0, 0, 0, 0.35);
  font: 0.92rem/1.4 var(--font-serif);
}
.c-papel { background: var(--map-papel); }
.c-amarelo { background: var(--map-amarelo); }
.c-verde { background: var(--map-verde); }
.c-azul { background: var(--map-azul); }
.c-rosa { background: var(--map-rosa); }
:global(.vue-flow__node.selected) .card { border-color: var(--ribbon); box-shadow: 0 0 0 1px var(--ribbon), 0 6px 16px -10px rgba(0, 0, 0, 0.35); }
.text { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; }
.text.empty { color: var(--note); font-style: italic; }
textarea.text {
  display: block;
  width: 100%;
  border: 0;
  padding: 0;
  background: transparent;
  resize: none;
  outline: none;
  font: inherit;
}
.badge {
  display: inline-block;
  margin-bottom: 0.3rem;
  padding: 0.05rem 0.45rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--ink) 10%, transparent);
  color: var(--note);
  font: 0.7rem/1.5 var(--font-ui);
  letter-spacing: 0.02em;
}
.more {
  position: absolute;
  top: 0.35rem;
  right: 0.3rem;
  display: grid;
  place-items: center;
  width: 1.5rem;
  height: 1.5rem;
  border: 0;
  border-radius: 6px;
  background: none;
  color: var(--note);
  cursor: pointer;
  opacity: 0;
}
.card:hover .more, .more:focus-visible, .card:has(.menu) .more { opacity: 1; }
.more:hover { background: color-mix(in srgb, var(--ink) 8%, transparent); }
.dot { width: 9px; height: 9px; border: 2px solid var(--sheet); background: var(--note); }
.card:hover .dot { background: var(--ribbon); }
.menu {
  position: absolute;
  top: 2rem;
  right: 0;
  z-index: 30;
  display: flex;
  flex-direction: column;
  min-width: 13rem;
  padding: 0.3rem;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 10px;
  background: #262b36;
  box-shadow: 0 16px 32px -12px rgba(0, 0, 0, 0.6);
  font: 0.82rem/1.4 var(--font-ui);
}
.colors { display: flex; gap: 0.4rem; padding: 0.35rem 0.5rem 0.5rem; }
.swatch { width: 1.2rem; height: 1.2rem; border: 1px solid rgba(255, 255, 255, 0.25); border-radius: 50%; cursor: pointer; }
.swatch.on { outline: 2px solid #e6e8ec; outline-offset: 1px; }
.menu-item { padding: 0.4rem 0.5rem; border: 0; border-radius: 6px; background: none; color: #e6e8ec; text-align: left; cursor: pointer; }
.menu-item:hover:not(:disabled), .menu-item:focus-visible { background: rgba(255, 255, 255, 0.08); }
.menu-item:disabled { opacity: 0.45; cursor: default; }
.menu-item.danger { color: #ff8a80; }
</style>
