<script setup lang="ts">
import VaultLayout from "../../components/VaultLayout.vue";
import { onMounted, ref } from "vue";
import { vaultNickname, vaultUnlock } from "../../lib/api";
import { errorMessage } from "../../lib/errors";

const emit = defineEmits<{ done: []; forgot: [] }>();
const password = ref("");
const error = ref("");
const busy = ref(false);
const nickname = ref<string | null>(null);

onMounted(async () => {
  nickname.value = await vaultNickname().catch(() => null);
});

async function unlock() {
  if (!password.value) return;
  busy.value = true;
  error.value = "";
  try {
    await vaultUnlock(password.value);
    emit("done");
  } catch (e) {
    error.value = errorMessage(e);
    password.value = "";
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <VaultLayout>
    <div v-if="nickname" class="who">
      <span class="avatar" aria-hidden="true">{{ nickname.slice(0, 1).toUpperCase() }}</span>
      <span><small>Entrando como</small><strong>{{ nickname }}</strong></span>
    </div>
    <form @submit.prevent="unlock">
      <label class="field"><span>Senha</span><input v-model="password" type="password" autocomplete="current-password" autofocus /></label>
      <p v-if="error" class="error">{{ error }}</p>
      <button class="btn wide" :disabled="!password || busy">{{ busy ? "Abrindo…" : "Abrir meus livros" }}</button>
      <div class="actions center">
        <button type="button" class="link" @click="emit('forgot')">Esqueci minha senha</button>
      </div>
    </form>
  </VaultLayout>
</template>

<style scoped>
.who {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin: -0.5rem 0 1.5rem;
  padding: 0.7rem 0.8rem;
  border: 1px solid var(--rule);
  border-radius: 10px;
  background: #fff;
}
.avatar {
  display: grid;
  place-items: center;
  width: 2.25rem;
  height: 2.25rem;
  border-radius: 50%;
  background: var(--ink);
  color: var(--paper);
  font: 400 1.1rem var(--font-mono);
}
.who small { display: block; font-size: 0.75rem; color: var(--note); }
.who strong { font-weight: 500; }
.center { justify-content: center; margin-top: 0.75rem; }
</style>
