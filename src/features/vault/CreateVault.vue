<script setup lang="ts">
import VaultLayout from "../../components/VaultLayout.vue";
import { computed, ref } from "vue";
import { vaultCreate } from "../../lib/api";
import { errorMessage } from "../../lib/errors";

const emit = defineEmits<{ done: [] }>();
const nickname = ref("");
const password = ref("");
const confirm = ref("");
const phrase = ref<string[] | null>(null);
const wrote = ref(false);
const error = ref("");
const busy = ref(false);

const problem = computed(() => {
  if (!nickname.value.trim()) return "Escolha um apelido.";
  if (password.value.length < 8) return "A senha precisa de pelo menos 8 caracteres.";
  if (password.value !== confirm.value) return "As duas senhas não são iguais.";
  return "";
});

async function create() {
  if (problem.value) return;
  busy.value = true;
  error.value = "";
  try {
    phrase.value = (await vaultCreate(nickname.value.trim(), password.value)).split(" ");
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <VaultLayout :wide="!!phrase">
    <template v-if="!phrase">
      <h1>Crie seu cofre</h1>
      <p class="lede">Tudo o que você escrever aqui fica cifrado neste computador. Só a sua senha abre.</p>
      <form @submit.prevent="create">
        <label class="field"><span>Apelido</span><input v-model="nickname" autocomplete="username" autofocus /></label>
        <label class="field"><span>Senha</span><input v-model="password" type="password" autocomplete="new-password" /></label>
        <label class="field"><span>Repita a senha</span><input v-model="confirm" type="password" autocomplete="new-password" /></label>
        <p v-if="error" class="error">{{ error }}</p>
        <p v-else-if="password && confirm && problem" class="error">{{ problem }}</p>
        <button class="btn wide" :disabled="!!problem || busy">Criar cofre</button>
      </form>
    </template>

    <template v-else>
      <h1>Suas 24 palavras</h1>
      <p class="lede">
        Guarde estas palavras em papel. Sem elas e sem sua senha, ninguém — nem o administrador — consegue
        recuperar seus livros.
      </p>
      <ol class="words">
        <li v-for="(w, i) in phrase" :key="i">{{ w }}</li>
      </ol>
      <label class="check"><input v-model="wrote" type="checkbox" /> Anotei as palavras</label>
      <div class="actions"><button class="btn wide" :disabled="!wrote" @click="emit('done')">Começar a escrever</button></div>
    </template>
  </VaultLayout>
</template>

<style scoped>
.words {
  columns: 3;
  background: #fff;
  border: 1px solid var(--rule);
  border-radius: 10px;
  padding: 1rem 1rem 1rem 2.75rem;
  column-gap: 2rem;
  font-family: var(--font-mono);
  font-size: 1.05rem;
  margin: 0 0 1.5rem;
  line-height: 1.9;
}
.words li::marker { color: var(--note); font-size: 0.8em; }
.check { display: flex; gap: 0.5rem; align-items: center; }
.check input { accent-color: var(--ribbon); width: 1.1rem; height: 1.1rem; }
@media (max-width: 30rem) { .words { columns: 2; } }
</style>
