<script setup lang="ts">
import { ref } from "vue";
import { vaultUnlock } from "../../lib/api";
import { errorMessage } from "../../lib/errors";

const emit = defineEmits<{ done: []; forgot: [] }>();
const password = ref("");
const error = ref("");
const busy = ref(false);

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
  <main class="sheet">
    <h1>De volta ao papel</h1>
    <p class="lede">Digite sua senha para abrir seus livros.</p>
    <form @submit.prevent="unlock">
      <label class="field"><span>Senha</span><input v-model="password" type="password" autocomplete="current-password" autofocus /></label>
      <p v-if="error" class="error">{{ error }}</p>
      <div class="actions">
        <button class="btn" :disabled="!password || busy">{{ busy ? "Abrindo…" : "Abrir" }}</button>
        <button type="button" class="link" @click="emit('forgot')">Esqueci minha senha</button>
      </div>
    </form>
  </main>
</template>
