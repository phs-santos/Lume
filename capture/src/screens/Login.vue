<script setup lang="ts">
import { ref } from "vue";
import { login } from "../api";
import { saveSession } from "../session";
import Shell from "./Shell.vue";

const nickname = ref("");
const password = ref("");
const error = ref("");
const busy = ref(false);

async function submit() {
  if (!nickname.value.trim() || !password.value) return;
  busy.value = true;
  error.value = "";
  try {
    const r = await login(nickname.value.trim(), password.value);
    saveSession(nickname.value.trim(), r.token, r.publicKey);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    password.value = "";
    busy.value = false;
  }
}
</script>

<template>
  <Shell>
    <form @submit.prevent="submit">
      <label class="field"><span>Apelido</span><input v-model="nickname" autocomplete="username" autocapitalize="none" /></label>
      <label class="field"><span>Senha (a mesma do Lume no computador)</span><input v-model="password" type="password" autocomplete="current-password" /></label>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
      <button class="btn" :disabled="busy || !nickname || !password">{{ busy ? "Entrando…" : "Entrar" }}</button>
    </form>
  </Shell>
</template>
