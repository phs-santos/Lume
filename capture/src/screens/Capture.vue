<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { fetchPublicKey, logout, postEnvelope } from "../api";
import { seal } from "../crypto";
import { AuthExpired, discardFailed, enqueue, failed, flush, pending, type Failed } from "../queue";
import { clearSession, saveSession, session } from "../session";
import Shell from "./Shell.vue";

const recording = ref(false);
const seconds = ref(0);
const text = ref("");
const waiting = ref(0);
const toast = ref("");
const error = ref("");
const rejected = ref<Failed[]>([]);
let recorder: MediaRecorder | null = null;
let chunks: Blob[] = [];
let tick: ReturnType<typeof setInterval> | null = null;
let retry: ReturnType<typeof setInterval> | null = null;

const clock = computed(() => `${Math.floor(seconds.value / 60)}:${String(seconds.value % 60).padStart(2, "0")}`);

function pickMime(): string {
  for (const m of ["audio/webm;codecs=opus", "audio/mp4", "audio/webm"]) if (MediaRecorder.isTypeSupported?.(m)) return m;
  return "";
}

function say(msg: string) {
  toast.value = msg;
  setTimeout(() => toast.value === msg && (toast.value = ""), 3500);
}

const mine = async () => (await pending()).filter((p) => p.nickname === session.nickname).length;

/** Confere se o computador trocou de chave; se trocou, as próximas ideias já vão para a nova. */
async function refreshKey() {
  try {
    const pk = await fetchPublicKey(session.token);
    if (session.publicKey && pk.join() !== session.publicKey.join()) {
      saveSession(session.nickname, session.token, pk);
      say("O Lume do computador trocou de chave. As próximas ideias já vão para a chave nova.");
    }
  } catch {
    /* sem rede: segue com a chave guardada */
  }
}

async function send() {
  try {
    await flush(session.nickname, (p) => postEnvelope(session.token, p));
  } catch (e) {
    // sessão expirada: pede login de novo; as ideias continuam guardadas no aparelho
    if (e instanceof AuthExpired) clearSession();
  }
  waiting.value = await mine();
  rejected.value = (await failed()).filter((f) => f.nickname === session.nickname);
  return { left: waiting.value };
}

async function discard(id: string) {
  await discardFailed(id);
  rejected.value = rejected.value.filter((f) => f.id !== id);
}

async function signOut() {
  const left = await mine();
  if (left && !confirm(`${left} ${left === 1 ? "ideia ainda não foi enviada" : "ideias ainda não foram enviadas"}. Elas ficam guardadas e sobem quando você entrar de novo. Sair mesmo assim?`)) return;
  await logout(session.token);
  clearSession();
}

/** Cifra no aparelho, guarda na fila e tenta enviar. */
async function keep(kind: "audio" | "text", mime: string, plain: Uint8Array) {
  const body = await seal(plain, session.publicKey!);
  await enqueue({ kind, mime, body, createdAt: Date.now(), nickname: session.nickname });
  const r = await send();
  say(r.left === 0 ? "Ideia enviada para o Lume." : "Ideia guardada. Envio quando houver rede.");
}

async function toggleRecording() {
  error.value = "";
  if (recording.value) return void recorder?.stop();
  try {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    const mime = pickMime();
    recorder = new MediaRecorder(stream, mime ? { mimeType: mime } : undefined);
    chunks = [];
    recorder.ondataavailable = (e) => e.data.size && chunks.push(e.data);
    recorder.onstop = async () => {
      stream.getTracks().forEach((t) => t.stop());
      recording.value = false;
      if (tick) clearInterval(tick);
      const type = recorder!.mimeType || mime || "audio/webm";
      const blob = new Blob(chunks, { type });
      await keep("audio", type, new Uint8Array(await blob.arrayBuffer()));
    };
    recorder.start();
    recording.value = true;
    seconds.value = 0;
    tick = setInterval(() => seconds.value++, 1000);
  } catch {
    error.value = "Não consegui usar o microfone. Permita o acesso nas configurações do navegador.";
  }
}

async function sendText() {
  const t = text.value.trim();
  if (!t) return;
  await keep("text", "text/plain;charset=utf-8", new TextEncoder().encode(t));
  text.value = "";
}

onMounted(async () => {
  // pede ao navegador para não apagar a fila de ideias quando faltar espaço
  navigator.storage?.persist?.().catch(() => {});
  await refreshKey();
  await send();
  window.addEventListener("online", send);
  retry = setInterval(send, 30_000);
});
onBeforeUnmount(() => {
  window.removeEventListener("online", send);
  if (retry) clearInterval(retry);
});
</script>

<template>
  <Shell>
    <section class="record">
      <button :class="['mic', { on: recording }]" :aria-label="recording ? 'Parar e enviar' : 'Gravar uma ideia'" @click="toggleRecording">
        <span class="dot" />
      </button>
      <p class="hint">{{ recording ? `Gravando ${clock} · toque para parar e enviar` : "Toque para gravar uma ideia" }}</p>
      <p v-if="error" class="error">{{ error }}</p>
    </section>

    <form class="write" @submit.prevent="sendText">
      <label class="field"><span>Ou escreva uma ideia</span><textarea v-model="text" placeholder="E se a Marina encontrar a carta antes do casamento?" /></label>
      <button class="btn" :disabled="!text.trim()">Enviar ideia</button>
    </form>

    <section v-if="rejected.length" class="rejected">
      <p>{{ rejected.length === 1 ? "Uma ideia não pôde ser enviada:" : `${rejected.length} ideias não puderam ser enviadas:` }}</p>
      <ul>
        <li v-for="f in rejected" :key="f.id">
          <span>{{ new Date(f.createdAt).toLocaleString("pt-BR", { day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit" }) }} — {{ f.reason }}</span>
          <button class="link" @click="discard(f.id)">Descartar</button>
        </li>
      </ul>
    </section>

    <footer>
      <span :class="['state', { wait: waiting }]">{{ waiting ? `${waiting} ${waiting === 1 ? "ideia esperando" : "ideias esperando"} rede` : "Tudo enviado" }}</span>
      <button class="link" @click="signOut">Sair ({{ session.nickname }})</button>
    </footer>

    <p v-if="toast" class="toast" role="status">{{ toast }}</p>
  </Shell>
</template>

<style scoped>
.record { display: flex; flex-direction: column; align-items: center; margin: 0.5rem 0 1.75rem; }
.mic {
  width: 7.5rem;
  height: 7.5rem;
  border-radius: 50%;
  border: 6px solid #e9e4d8;
  background: radial-gradient(circle at 50% 40%, #d0352a, #8f1b12);
  box-shadow: 0 10px 24px -8px rgba(180, 35, 24, 0.7), inset 0 -6px 10px rgba(0, 0, 0, 0.25);
  display: grid;
  place-items: center;
  cursor: pointer;
}
.mic .dot { width: 2rem; height: 2rem; border-radius: 50%; background: #fff; transition: border-radius 0.2s, transform 0.2s; }
.mic.on { animation: pulse 1.6s ease-in-out infinite; }
.mic.on .dot { border-radius: 6px; transform: scale(0.85); }
@keyframes pulse { 50% { box-shadow: 0 0 0 14px rgba(180, 35, 24, 0.15), inset 0 -6px 10px rgba(0, 0, 0, 0.25); } }
.hint { color: var(--note); font-size: 0.9rem; margin: 0.9rem 0 0; text-align: center; }
.write { margin-bottom: 1.25rem; }
.rejected { margin: 0 0 1rem; padding: 0.75rem 0.9rem; border: 1px solid #f0c2bd; border-radius: 10px; background: #fff5f4; font-size: 0.85rem; }
.rejected p { margin: 0 0 0.4rem; color: var(--ribbon); }
.rejected ul { list-style: none; margin: 0; padding: 0; }
.rejected li { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; }
footer { display: flex; justify-content: space-between; align-items: center; font-size: 0.85rem; }
.state { color: #3c8a5f; }
.state.wait { color: #b07a10; }
.toast {
  position: fixed;
  left: 1rem;
  right: 1rem;
  bottom: calc(1rem + env(safe-area-inset-bottom));
  margin: 0;
  padding: 0.85rem 1rem;
  background: var(--ink);
  color: #fff;
  border-radius: 12px;
  text-align: center;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
}
</style>
