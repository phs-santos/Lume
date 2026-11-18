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
const error = ref("");
const busy = ref(false);
const copied = ref(false);

/** Conferência: duas palavras sorteadas que a pessoa digita de volta. */
const step = ref<"show" | "check">("show");
const asked = ref<number[]>([]);
const answers = ref(["", ""]);

const problem = computed(() => {
  if (!nickname.value.trim()) return "Escolha um nome para o cofre.";
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

async function copy() {
  if (!phrase.value) return;
  try {
    await navigator.clipboard.writeText(phrase.value.map((w, i) => `${i + 1}. ${w}`).join("\n"));
    copied.value = true;
    setTimeout(() => (copied.value = false), 2000);
  } catch {
    error.value = "Não consegui copiar. Anote à mão.";
  }
}

const print = () => window.print();

function startCheck() {
  const n = phrase.value!.length;
  const a = Math.floor(Math.random() * n);
  let b = Math.floor(Math.random() * (n - 1));
  if (b >= a) b++;
  asked.value = [a, b].sort((x, y) => x - y);
  answers.value = ["", ""];
  step.value = "check";
}

const norm = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").trim().toLowerCase();
const checked = computed(() => asked.value.every((i, k) => norm(answers.value[k]) === phrase.value![i]));
</script>

<template>
  <VaultLayout :wide="!!phrase">
    <div v-if="!phrase" key="form">
      <h1>Crie seu cofre</h1>
      <p class="lede">
        O cofre guarda seus livros neste computador, trancados com uma senha. Ninguém lê sem ela, nem quem fez o Lume.
      </p>
      <form @submit.prevent="create">
        <label class="field">
          <span>Seu nome ou apelido</span>
          <input v-model="nickname" autocomplete="username" autofocus />
          <small class="help">Só para identificar o cofre. Não é o nome de autor nos livros.</small>
        </label>
        <label class="field">
          <span>Senha</span>
          <input v-model="password" type="password" autocomplete="new-password" aria-describedby="pw-rule" />
          <small id="pw-rule" :class="['help', { ok: password.length >= 8 }]">
            {{ password.length >= 8 ? "✓ " : "" }}Pelo menos 8 caracteres
          </small>
        </label>
        <label class="field"><span>Repita a senha</span><input v-model="confirm" type="password" autocomplete="new-password" /></label>
        <p v-if="error" class="error">{{ error }}</p>
        <p v-else-if="(nickname || password || confirm) && problem" class="why" data-test="why">{{ problem }}</p>
        <button class="btn wide" :disabled="!!problem || busy">Criar cofre</button>
      </form>
    </div>

    <div v-else-if="step === 'show'" key="show">
      <h1>Suas palavras de recuperação</h1>
      <p class="lede">
        Se um dia você esquecer a senha, só estas 24 palavras abrem o cofre de novo, na tela “Esqueci a senha”.
        Sem a senha e sem elas, os livros ficam perdidos para sempre. Guarde num lugar seguro, fora do computador.
      </p>
      <ol class="words">
        <li v-for="(w, i) in phrase" :key="i">{{ w }}</li>
      </ol>
      <div class="tools">
        <button class="btn quiet" data-test="copy" @click="copy">{{ copied ? "Copiado ✓" : "Copiar" }}</button>
        <button class="btn quiet" data-test="print" @click="print">Imprimir</button>
      </div>
      <p v-if="error" class="error">{{ error }}</p>
      <div class="actions"><button class="btn wide" data-test="to-check" @click="startCheck">Já guardei as palavras</button></div>
    </div>

    <div v-else key="check">
      <h1>Confira</h1>
      <p class="lede">Para ter certeza de que anotou certo, digite duas delas.</p>
      <form @submit.prevent="checked && emit('done')">
        <label v-for="(i, k) in asked" :key="i" class="field">
          <span>Palavra {{ i + 1 }}</span>
          <input v-model="answers[k]" :data-test="`answer-${k}`" autocomplete="off" autocapitalize="off" spellcheck="false" />
        </label>
        <p v-if="answers.every((a) => a.trim()) && !checked" class="why">Alguma palavra não confere. Olhe de novo a sua anotação.</p>
        <button class="btn wide" data-test="done" :disabled="!checked">Começar a escrever</button>
      </form>
      <button class="link back" @click="step = 'show'">Ver as palavras de novo</button>
    </div>
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
  margin: 0 0 1rem;
  line-height: 1.9;
}
.words li::marker { color: var(--note); font-size: 0.8em; }
.tools { display: flex; gap: 0.5rem; margin-bottom: 1.25rem; }
.help { display: block; margin-top: 0.3rem; font-size: 0.78rem; color: var(--note); }
.help.ok { color: #2f7a52; }
.why { margin: 0 0 1rem; font-size: 0.85rem; color: var(--note); }
.back { margin-top: 0.75rem; }
@media (max-width: 30rem) { .words { columns: 2; } }
/* imprimir: só as palavras */
@media print {
  :global(body *) { visibility: hidden; }
  .words, .words * { visibility: visible; }
  .words { position: fixed; top: 2rem; left: 2rem; columns: 2; border: 0; }
}
</style>
