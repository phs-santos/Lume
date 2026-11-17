<script setup lang="ts">
import { computed, ref } from "vue";
import { vaultRecover } from "../../lib/api";
import { errorMessage } from "../../lib/errors";

const emit = defineEmits<{ done: []; back: [] }>();
const phrase = ref("");
const password = ref("");
const confirm = ref("");
const error = ref("");
const busy = ref(false);

const wordCount = computed(() => phrase.value.trim().split(/\s+/).filter(Boolean).length);
const problem = computed(() => {
  if (wordCount.value !== 24) return `São 24 palavras — você digitou ${wordCount.value}.`;
  if (password.value.length < 8) return "A nova senha precisa de pelo menos 8 caracteres.";
  if (password.value !== confirm.value) return "As duas senhas não são iguais.";
  return "";
});

async function recover() {
  if (problem.value) return;
  busy.value = true;
  error.value = "";
  try {
    await vaultRecover(phrase.value, password.value);
    emit("done");
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <main class="sheet">
    <h1>Recuperar o cofre</h1>
    <p class="lede">Digite as 24 palavras que você anotou e escolha uma senha nova.</p>
    <form @submit.prevent="recover">
      <label class="field"><span>As 24 palavras, separadas por espaço</span><textarea v-model="phrase" spellcheck="false" autofocus /></label>
      <label class="field"><span>Nova senha</span><input v-model="password" type="password" autocomplete="new-password" /></label>
      <label class="field"><span>Repita a nova senha</span><input v-model="confirm" type="password" autocomplete="new-password" /></label>
      <p v-if="error" class="error">{{ error }}</p>
      <p v-else-if="phrase && confirm && problem" class="error">{{ problem }}</p>
      <div class="actions">
        <button class="btn" :disabled="!!problem || busy">Recuperar e abrir</button>
        <button type="button" class="link" @click="emit('back')">Voltar</button>
      </div>
    </form>
  </main>
</template>
