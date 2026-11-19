<script setup lang="ts">
import { computed, nextTick, onMounted, provide, ref, shallowRef, watch } from "vue";
import { ConnectionMode, VueFlow, useVueFlow, type Connection, type Edge, type EdgeChange, type EdgeMouseEvent, type Node, type NodeChange, type NodeDragEvent } from "@vue-flow/core";
import "@vue-flow/core/dist/style.css";
import { IconFocusCentered, IconPlus } from "@tabler/icons-vue";
import { bibleList, mapCardCreate, mapCardDelete, mapCardMove, mapCardToBible, mapCardUpdate, mapGet, mapLinkCreate, mapLinkDelete, mapLinkLabel } from "../../lib/api";
import type { BibleEntry, CardColor, MapCard } from "../../lib/types";
import CardNode from "./CardNode.vue";
import { MAP_ACTIONS } from "./actions";
import { CARD_H, CARD_W, newCardsBounds, toEdges, toNodes, type CardData } from "./flow";

const props = defineProps<{ bookId: string }>();
const emit = defineEmits<{ bibleChanged: [] }>();

const flowId = `mapa-${props.bookId}`;
const { fitView, fitBounds, screenToFlowCoordinate, viewport, dimensions } = useVueFlow(flowId);

const cards = ref<MapCard[]>([]);
const nodes = shallowRef<Node[]>([]);
const edges = shallowRef<Edge[]>([]);
const kinds = ref<Record<string, string>>({});
const loaded = ref(false);
const error = ref("");
const editRequest = ref<string | null>(null);
const label = ref<{ id: string; text: string; x: number; y: number } | null>(null);
const labelBox = ref<HTMLInputElement | null>(null);
const root = ref<HTMLElement | null>(null);

async function attempt(fn: () => Promise<unknown>) {
  error.value = "";
  try {
    await fn();
  } catch (e) {
    error.value = `Não deu para salvar no mapa: ${e}`;
  }
}

function setCards(list: MapCard[]) {
  cards.value = list;
  nodes.value = toNodes(list);
}

async function loadKinds() {
  const list: BibleEntry[] = await bibleList(props.bookId);
  kinds.value = Object.fromEntries(list.map((e) => [e.id, e.kind]));
}

async function reload() {
  const m = await mapGet(props.bookId);
  setCards(m.cards);
  edges.value = toEdges(m.links);
  await loadKinds();
  loaded.value = true;
}

function patchCard(c: MapCard) {
  cards.value = cards.value.map((x) => (x.id === c.id ? c : x));
  nodes.value = nodes.value.map((n) => (n.id === c.id ? { ...n, data: { card: c } satisfies CardData } : n));
}

provide(MAP_ACTIONS, {
  save: (id, text, color) => attempt(async () => patchCard(await mapCardUpdate(id, text, color))),
  remove: (id) => attempt(async () => {
    await mapCardDelete(id);
    setCards(cards.value.filter((c) => c.id !== id));
    edges.value = edges.value.filter((e) => e.source !== id && e.target !== id);
  }),
  toBible: (id, kind) => attempt(async () => {
    const e = await mapCardToBible(id, kind);
    kinds.value = { ...kinds.value, [e.id]: e.kind };
    const c = cards.value.find((x) => x.id === id);
    if (c) patchCard({ ...c, bible_entry_id: e.id });
    emit("bibleChanged");
  }),
  kindOf: (entryId) => kinds.value[entryId] ?? null,
  consumeEditRequest: (id) => {
    if (editRequest.value !== id) return false;
    editRequest.value = null;
    return true;
  },
});

async function addCardAt(x: number, y: number, color: CardColor = "papel") {
  await attempt(async () => {
    const c = await mapCardCreate(props.bookId, "", color, x, y);
    editRequest.value = c.id;
    setCards([...cards.value, c]);
  });
}

/** "+ Cartão": no meio do que está à vista. */
function addCardCentered() {
  const { width, height } = dimensions.value;
  const { x, y, zoom } = viewport.value;
  void addCardAt((width / 2 - x) / zoom - CARD_W / 2, (height / 2 - y) / zoom - CARD_H / 2);
}

function onDblClick(e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (!t.closest(".vue-flow__pane") || t.closest(".vue-flow__node, .vue-flow__edge")) return;
  const p = screenToFlowCoordinate({ x: e.clientX, y: e.clientY });
  void addCardAt(p.x - CARD_W / 2, p.y - 24);
}

function onDragStop(e: NodeDragEvent) {
  for (const n of e.nodes) {
    const c = cards.value.find((x) => x.id === n.id);
    if (!c || (c.x === n.position.x && c.y === n.position.y)) continue;
    patchCard({ ...c, x: n.position.x, y: n.position.y });
    void attempt(() => mapCardMove(n.id, n.position.x, n.position.y));
  }
}

function onConnect(c: Connection) {
  if (!c.source || !c.target || c.source === c.target) return;
  void attempt(async () => {
    const l = await mapLinkCreate(props.bookId, c.source, c.target);
    if (!edges.value.some((e) => e.id === l.id)) edges.value = [...edges.value, ...toEdges([l])];
  });
}

/** Delete/Backspace com algo selecionado: o Vue Flow tira da tela; aqui apaga do cofre. */
function onNodesChange(changes: NodeChange[]) {
  for (const ch of changes) if (ch.type === "remove") void attempt(async () => {
    await mapCardDelete(ch.id);
    setCards(cards.value.filter((c) => c.id !== ch.id));
  });
}
function onEdgesChange(changes: EdgeChange[]) {
  for (const ch of changes) if (ch.type === "remove") void attempt(() => mapLinkDelete(ch.id));
}

async function onEdgeDblClick({ edge, event }: EdgeMouseEvent) {
  const box = root.value!.getBoundingClientRect();
  const ev = event as MouseEvent;
  label.value = { id: edge.id, text: String(edge.label ?? ""), x: ev.clientX - box.left, y: ev.clientY - box.top };
  await nextTick();
  labelBox.value?.focus();
  labelBox.value?.select();
}
async function saveLabel() {
  const l = label.value;
  if (!l) return;
  label.value = null;
  await attempt(async () => {
    await mapLinkLabel(l.id, l.text);
    edges.value = edges.value.map((e) => (e.id === l.id ? { ...e, label: l.text.trim() } : e));
  });
}

/** Enquadra os cartões que acabaram de entrar. */
async function focusCards(ids: string[]) {
  await nextTick();
  const b = newCardsBounds(cards.value, ids);
  if (b) await fitBounds(b, { padding: 0.35, duration: 300 });
}

const empty = computed(() => loaded.value && cards.value.length === 0);

onMounted(async () => {
  await attempt(reload);
  await nextTick();
  if (cards.value.length) await fitView({ padding: 0.25, maxZoom: 1 });
});
watch(() => props.bookId, () => void attempt(reload));

defineExpose({ reload, focusCards });
</script>

<template>
  <section ref="root" class="map" aria-label="Mapa de ideias" @dblclick="onDblClick">
    <VueFlow
      :id="flowId"
      v-model:nodes="nodes"
      v-model:edges="edges"
      :zoom-on-double-click="false"
      :delete-key-code="['Backspace', 'Delete']"
      multi-selection-key-code="Shift"
      :connection-mode="ConnectionMode.Loose"
      :min-zoom="0.2"
      :max-zoom="2"
      :apply-default="true"
      @node-drag-stop="onDragStop"
      @connect="onConnect"
      @nodes-change="onNodesChange"
      @edges-change="onEdgesChange"
      @edge-double-click="onEdgeDblClick"
    >
      <template #node-card="p"><CardNode v-bind="p" /></template>
    </VueFlow>

    <div class="tools">
      <button class="tool" title="Novo cartão" @click="addCardCentered"><IconPlus :size="16" stroke-width="2" /><span>Cartão</span></button>
      <button class="tool" title="Ver o mapa todo" :disabled="!cards.length" @click="fitView({ padding: 0.25, duration: 300, maxZoom: 1 })">
        <IconFocusCentered :size="16" stroke-width="2" /><span>Ver tudo</span>
      </button>
    </div>

    <p v-if="empty" class="hint">Dê dois cliques para criar um cartão. Puxe a bolinha de um cartão até outro para ligar.</p>
    <p v-if="error" class="error" role="alert">{{ error }}</p>

    <input
      v-if="label"
      ref="labelBox"
      v-model="label.text"
      class="label-box"
      :style="{ left: `${label.x}px`, top: `${label.y}px` }"
      placeholder="Rótulo da seta"
      aria-label="Rótulo da seta"
      maxlength="60"
      @keydown.enter.prevent="saveLabel"
      @keydown.esc.prevent="saveLabel"
      @blur="saveLabel"
    />
  </section>
</template>

<style scoped>
.map {
  position: relative;
  height: 100%;
  min-height: 0;
  background:
    radial-gradient(circle, color-mix(in srgb, var(--ink) 14%, transparent) 1px, transparent 1.2px) 0 0 / 22px 22px,
    var(--desk);
}
.map :deep(.vue-flow__edge-path) { stroke: color-mix(in srgb, var(--ink) 55%, transparent); stroke-width: 1.5; }
.map :deep(.vue-flow__edge.selected .vue-flow__edge-path) { stroke: var(--ribbon); }
.map :deep(.vue-flow__edge-textbg) { fill: var(--desk); }
.map :deep(.vue-flow__edge-text) { fill: var(--note); font: 0.78rem var(--font-ui); }
.map :deep(.vue-flow__arrowhead polyline) { stroke: color-mix(in srgb, var(--ink) 55%, transparent); fill: color-mix(in srgb, var(--ink) 55%, transparent); }
.map :deep(.vue-flow__connection-path) { stroke: var(--ribbon); stroke-width: 1.5; }
.map :deep(.vue-flow__selection) { background: color-mix(in srgb, var(--ribbon) 8%, transparent); border: 1px dashed var(--ribbon); }
.map :deep(.vue-flow__node) { cursor: grab; }
.map :deep(.vue-flow__handle) { width: 11px; height: 11px; border: 2px solid var(--sheet); border-radius: 50%; background: var(--note); }
.map :deep(.vue-flow__node:hover .vue-flow__handle), .map :deep(.vue-flow__handle.connecting) { background: var(--ribbon); }

.tools { position: absolute; top: 0.9rem; left: 0.9rem; display: flex; gap: 0.4rem; z-index: 5; }
.tool {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.35rem 0.7rem;
  border: 1px solid var(--rule);
  border-radius: 8px;
  background: var(--sheet);
  color: var(--ink);
  font-size: 0.84rem;
  cursor: pointer;
}
.tool:hover:not(:disabled) { border-color: var(--note); }
.tool:disabled { opacity: 0.5; cursor: default; }

.hint {
  position: absolute;
  inset: 50% auto auto 50%;
  transform: translate(-50%, -50%);
  max-width: 22rem;
  margin: 0;
  color: var(--note);
  text-align: center;
  pointer-events: none;
  font-size: 0.95rem;
}
.error {
  position: absolute;
  bottom: 1rem;
  left: 50%;
  transform: translateX(-50%);
  margin: 0;
  padding: 0.5rem 0.9rem;
  border-radius: 8px;
  background: #262b36;
  color: #ff8a80;
  font-size: 0.85rem;
  z-index: 6;
}
.label-box {
  position: absolute;
  z-index: 6;
  transform: translate(-50%, -50%);
  width: 12rem;
  padding: 0.3rem 0.5rem;
  border: 1px solid var(--ribbon);
  border-radius: 6px;
  background: var(--sheet);
  font-size: 0.85rem;
}
</style>
