<script setup lang="ts">
import { IconArchive, IconArrowBackUp, IconMessageCircle, IconPlugConnectedX, IconFilePlus, IconRowInsertBottom, IconMicrophone, IconPencil, IconPlayerPlay, IconRefresh, IconTrash, IconX } from "@tabler/icons-vue";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { inboxArchive, inboxBody, inboxDelete, inboxList, inboxRestore, inboxRetryTranscript, inboxToScene, inboxTranscribePending, remoteDisconnect, remoteStatus, remoteSync } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import type { InboxItem, RemoteStatus, SceneSummary } from "../../lib/types";
import { audioBlob } from "./audioUrl";
import ConnectPhone from "./ConnectPhone.vue";
import { aboutIdea, ideaText, transcriptLabel } from "./ideaText";
import PhoneQr from "./PhoneQr.vue";

const props = defineProps<{ current: SceneSummary | null; fallbackChapterId: string | null; insertIntoScene: (text: string) => boolean }>();
const emit = defineEmits<{ close: []; sceneCreated: [id: string]; changed: []; discuss: [text: string] }>();

const status = ref<RemoteStatus | null>(null);
const items = ref<InboxItem[]>([]);
const tab = ref<"nova" | "arquivada">("nova");
const busy = ref(false);
const message = ref("");
const audio = ref<Record<string, string>>({});
const confirming = ref<string | null>(null);
const expanded = ref<Record<string, boolean>>({});

const shown = computed(() => items.value.filter((i) => i.status === tab.value));
const counts = computed(() => ({
  nova: items.value.filter((i) => i.status === "nova").length,
  arquivada: items.value.filter((i) => i.status === "arquivada").length,
}));

const when = (t: number) =>
  new Date(t).toLocaleString("pt-BR", { day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit" });

async function load() {
  status.value = await remoteStatus();
  items.value = await inboxList();
  emit("changed");
}

async function fetchNow() {
  busy.value = true;
  message.value = "";
  try {
    const n = await remoteSync();
    message.value = n ? `${n} ${n === 1 ? "ideia nova chegou" : "ideias novas chegaram"}.` : "Nada novo por enquanto.";
  } catch (e) {
    message.value = errorMessage(e);
  } finally {
    busy.value = false;
    await load();
  }
  await transcribe();
}

/** Transcreve os áudios que chegaram (em segundo plano) e atualiza os cartões. */
async function transcribe() {
  if (!items.value.some((i) => i.kind === "audio" && (i.transcript_status === "pendente" || i.transcript_status === null))) return;
  try {
    await inboxTranscribePending();
  } catch (e) {
    message.value = errorMessage(e);
  }
  await load();
}

/** Inserida no fim da cena aberta: arquiva, para não entrar duas vezes. */
async function insertHere(i: InboxItem) {
  const text = ideaText(i);
  if (!text || !props.insertIntoScene(text)) return;
  await act(() => inboxArchive(i.id));
  message.value = "Ideia inserida no fim da cena e arquivada.";
}

async function retry(i: InboxItem) {
  await act(() => inboxRetryTranscript(i.id));
  await transcribe();
}

async function disconnect() {
  if (!confirm("Desconectar este computador? As ideias já recebidas continuam aqui.")) return;
  await remoteDisconnect();
  await load();
}

async function play(i: InboxItem) {
  audio.value[i.id] ??= URL.createObjectURL(audioBlob(await inboxBody(i.id), i.mime));
}

async function act(fn: () => Promise<unknown>) {
  message.value = "";
  try {
    await fn();
  } catch (e) {
    message.value = errorMessage(e);
  }
  await load();
}

async function toScene(i: InboxItem) {
  const chapter = props.current?.chapter_id ?? props.fallbackChapterId;
  if (!chapter) return void (message.value = "Abra uma cena para eu saber onde encaixar.");
  await act(async () => {
    const s = await inboxToScene(i.id, chapter, props.current ? props.current.position + 1 : undefined);
    emit("sceneCreated", s.id);
  });
}

onMounted(async () => {
  await load();
  if (status.value?.connected) await fetchNow();
});
const waiting = computed(() => items.value.some((i) => i.kind === "audio" && (i.transcript_status === "pendente" || i.transcript_status === null)));
const poll = setInterval(() => {
  if (waiting.value) void load();
}, 5000);

onBeforeUnmount(() => {
  clearInterval(poll);
  Object.values(audio.value).forEach(URL.revokeObjectURL);
});
</script>

<template>
  <aside class="drawer" data-popover aria-label="Ideias do celular">
    <header>
      <h2>Ideias</h2>
      <button v-if="status?.connected" class="link icon" :disabled="busy" title="Buscar agora" aria-label="Buscar ideias agora" @click="fetchNow">
        <IconRefresh :size="18" :class="{ spin: busy }" />
      </button>
      <button v-if="status?.connected" class="link icon" title="Desconectar este computador" aria-label="Desconectar este computador" @click="disconnect">
        <IconPlugConnectedX :size="18" />
      </button>
      <button class="link icon" aria-label="Fechar" @click="emit('close')"><IconX :size="18" /></button>
    </header>

    <ConnectPhone v-if="status && !status.connected" @connected="fetchNow" />

    <template v-else-if="status">
      <PhoneQr v-if="status.url && status.nickname" :url="status.url" :nickname="status.nickname" />
      <div class="tabs" role="tablist">
        <button role="tab" :aria-selected="tab === 'nova'" @click="tab = 'nova'">Novas <span>{{ counts.nova }}</span></button>
        <button role="tab" :aria-selected="tab === 'arquivada'" @click="tab = 'arquivada'">Arquivadas <span>{{ counts.arquivada }}</span></button>
      </div>
      <p v-if="message" class="message" role="status">{{ message }}</p>

      <p v-if="!shown.length" class="empty">
        <template v-if="tab === 'nova'">
          Nenhuma ideia nova. Grave ou escreva no celular, em <strong>{{ status.url?.replace("https://", "") }}</strong>.
        </template>
        <template v-else>Nada arquivado ainda.</template>
      </p>

      <ul class="cards">
        <li v-for="i in shown" :key="i.id" class="card">
          <div class="meta">
            <IconMicrophone v-if="i.kind === 'audio'" :size="16" />
            <IconPencil v-else :size="16" />
            <span>{{ when(i.created_at) }}</span>
          </div>
          <template v-if="i.kind === 'audio'">
            <audio v-if="audio[i.id]" :src="audio[i.id]" controls autoplay class="player" />
            <button v-else class="btn quiet play" @click="play(i)"><IconPlayerPlay :size="16" /> Ouvir</button>
          </template>
          <p v-if="ideaText(i)" :class="['text', { clamp: !expanded[i.id] }]">{{ ideaText(i) }}</p>
          <button v-if="(ideaText(i)?.length ?? 0) > 320" class="link more" @click="expanded[i.id] = !expanded[i.id]">
            {{ expanded[i.id] ? "Ver menos" : "Ver tudo" }}
          </button>
          <p v-if="transcriptLabel(i)" :class="['state', i.transcript_status]">
            {{ transcriptLabel(i) }}
            <button v-if="i.transcript_status === 'erro'" class="link" @click="retry(i)">Tentar de novo</button>
          </p>
          <div class="actions">
            <template v-if="ideaText(i) && i.status === 'nova'">
              <button class="link" @click="toScene(i)"><IconFilePlus :size="16" /> Virar cena aqui</button>
              <button class="link" @click="insertHere(i)"><IconRowInsertBottom :size="16" /> Inserir no fim da cena</button>
              <button class="link" @click="emit('discuss', aboutIdea(ideaText(i)!))"><IconMessageCircle :size="16" /> Conversar sobre isso</button>
            </template>
            <button v-if="i.status === 'nova'" class="link" @click="act(() => inboxArchive(i.id))"><IconArchive :size="16" /> Arquivar</button>
            <button v-else class="link" @click="act(() => inboxRestore(i.id))"><IconArrowBackUp :size="16" /> Voltar para novas</button>
            <template v-if="confirming === i.id">
              <span class="ask">Apagar de vez?</span>
              <button class="link danger" @click="act(() => inboxDelete(i.id)).then(() => (confirming = null))">Apagar</button>
              <button class="link" @click="confirming = null">Não</button>
            </template>
            <button v-else class="link icon" aria-label="Apagar ideia" @click="confirming = i.id"><IconTrash :size="16" /></button>
          </div>
        </li>
      </ul>
    </template>
  </aside>
</template>

<style scoped>
.drawer {
  position: absolute;
  top: 0.75rem;
  right: 0.75rem;
  bottom: 0.75rem;
  width: min(24rem, calc(100% - 1.5rem));
  display: flex;
  flex-direction: column;
  padding: 1rem 1.1rem;
  background: var(--paper);
  border-radius: 14px;
  box-shadow: 0 24px 50px -18px rgba(0, 0, 0, 0.55);
  z-index: 12;
  overflow-y: auto;
  scrollbar-width: thin;
}
header { display: flex; align-items: center; gap: 0.25rem; margin-bottom: 0.75rem; }
h2 { flex: 1; margin: 0; font: 400 1.4rem var(--font-mono); }
.icon { padding: 0 0.45rem; }
.spin { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.tabs { display: flex; gap: 0.25rem; padding: 3px; border: 1px solid var(--rule); border-radius: 10px; margin-bottom: 0.75rem; }
.tabs button { flex: 1; border: 0; border-radius: 7px; padding: 0.4rem; background: none; color: var(--note); cursor: pointer; }
.tabs button[aria-selected="true"] { background: var(--ink); color: var(--paper); }
.tabs span { opacity: 0.7; font-size: 0.8em; margin-left: 0.2rem; }
.message { font-size: 0.85rem; color: var(--note); margin: 0 0 0.75rem; }
.empty { color: var(--note); font-size: 0.9rem; text-align: center; margin: 2rem 0.5rem; }
.cards { list-style: none; margin: 0; padding: 0; display: grid; gap: 0.75rem; }
.card { padding: 0.85rem 0.9rem 0.5rem; border: 1px solid var(--rule); border-radius: 12px; background: var(--sheet); }
.meta { display: flex; gap: 0.4rem; align-items: center; color: var(--note); font-size: 0.78rem; margin-bottom: 0.4rem; }
.text { margin: 0 0 0.5rem; font: 1rem/1.55 var(--font-serif); white-space: pre-wrap; }
.player { width: 100%; margin-bottom: 0.25rem; }
.text.clamp { display: -webkit-box; -webkit-line-clamp: 6; -webkit-box-orient: vertical; overflow: hidden; }
.more { margin: -0.25rem 0 0.4rem -0.5rem; font-size: 0.8rem; }
.state { display: flex; align-items: center; flex-wrap: wrap; gap: 0.25rem; margin: 0.2rem 0 0.5rem; font-size: 0.82rem; color: var(--note); font-style: italic; }
.state.erro { color: var(--ribbon); font-style: normal; }
.play { margin-bottom: 0.25rem; }
.actions { display: flex; flex-wrap: wrap; align-items: center; gap: 0.1rem; margin: 0 -0.5rem; font-size: 0.82rem; }
.ask { color: var(--ribbon); padding: 0 0.4rem; }
.danger { color: var(--ribbon); }
</style>
