<script setup lang="ts">
import VaultLayout from "../../components/VaultLayout.vue";
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
  <VaultLayout>
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
.center { justify-content: center; margin-top: 0.75rem; }
</style>
