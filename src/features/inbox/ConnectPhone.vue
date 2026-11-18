<script setup lang="ts">
import { onMounted, ref } from "vue";
import { remoteConnect, vaultNickname } from "../../lib/api";
import { errorMessage } from "../../lib/errors";

const emit = defineEmits<{ connected: [] }>();
const url = ref("https://lume.codexolimpo.com");
const nickname = ref("");
const password = ref("");
const invite = ref("");
const firstTime = ref(false);
const busy = ref(false);
const error = ref("");

onMounted(async () => {
  nickname.value = (await vaultNickname().catch(() => null)) ?? "";
});

async function connect() {
  busy.value = true;
  error.value = "";
  try {
    await remoteConnect(url.value.trim(), nickname.value.trim(), password.value, firstTime.value ? invite.value.trim() : undefined);
    emit("connected");
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    password.value = "";
    busy.value = false;
  }
}
</script>

<template>
  <form class="connect" @submit.prevent="connect">
    <h3>Receber ideias do celular</h3>
    <p class="hint">
      Anote ideias e grave áudios no celular, e eles aparecem aqui, cifrados do começo ao fim.
      Primeiro conecte este computador; depois o Lume mostra um QR code para abrir no celular.
    </p>
    <label class="field"><span>Seu nome no cofre</span><input v-model="nickname" autocomplete="username" /></label>
    <label class="field"><span>Senha do Lume</span><input v-model="password" type="password" autocomplete="current-password" /></label>
    <label class="check"><input v-model="firstTime" type="checkbox" /> É a primeira vez que conecto (recebi um código de acesso)</label>
    <label v-if="firstTime" class="field"><span>Código de acesso</span><input v-model="invite" autocomplete="off" /></label>
    <details class="advanced">
      <summary>Avançado</summary>
      <label class="field"><span>Endereço</span><input v-model="url" /></label>
    </details>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <button class="btn wide" :disabled="busy || !nickname || !password || (firstTime && !invite)">
      {{ busy ? "Conectando…" : "Conectar este computador" }}
    </button>
  </form>
</template>

<style scoped>
h3 { margin: 0 0 0.4rem; font: 500 1rem var(--font-ui); }
.hint { color: var(--note); font-size: 0.85rem; margin: 0 0 1.25rem; }
.check { display: flex; gap: 0.5rem; align-items: center; font-size: 0.85rem; margin: -0.25rem 0 1rem; }
.check input { accent-color: var(--ribbon); }
.advanced { margin: 0 0 1rem; font-size: 0.85rem; color: var(--note); }
.advanced summary { cursor: pointer; margin-bottom: 0.5rem; }
</style>
