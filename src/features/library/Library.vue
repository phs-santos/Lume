<script setup lang="ts">
import { nextTick, onMounted, ref } from "vue";
import { bookCreate, bookList, vaultLock } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import type { AiMode, Book } from "../../lib/types";

const emit = defineEmits<{ open: [bookId: string]; locked: [] }>();
const books = ref<Book[]>([]);
const creating = ref(false);
const title = ref("");
const mode = ref<AiMode>("conselheira");
const error = ref("");
const titleInput = ref<HTMLInputElement | null>(null);

async function startCreating() {
  creating.value = true;
  await nextTick();
  titleInput.value?.focus();
}

const modes: { value: AiMode; name: string; hint: string }[] = [
  { value: "conselheira", name: "Conselheira", hint: "A IA só sugere. Nunca escreve no seu texto." },
  { value: "coautora", name: "Coautora", hint: "A IA escreve rascunhos que você aprova." },
];

async function load() {
  try {
    books.value = await bookList();
  } catch (e) {
    error.value = errorMessage(e);
  }
}

async function create() {
  if (!title.value.trim()) return;
  try {
    const b = await bookCreate(title.value.trim(), mode.value);
    emit("open", b.id);
  } catch (e) {
    error.value = errorMessage(e);
  }
}

async function lock() {
  await vaultLock();
  emit("locked");
}

onMounted(load);
</script>

<template>
  <main class="sheet">
    <h1>Seus livros</h1>
    <p v-if="error" class="error">{{ error }}</p>

    <ul v-if="books.length" class="books">
      <li v-for="b in books" :key="b.id">
        <button class="book" @click="emit('open', b.id)">
          <span class="title">{{ b.title }}</span>
          <span class="mode">{{ b.ai_mode === "coautora" ? "Coautora" : "Conselheira" }}</span>
        </button>
      </li>
    </ul>
    <p v-else-if="!creating" class="lede">Nenhum livro ainda. Comece o primeiro.</p>

    <form v-if="creating" class="new" @submit.prevent="create">
      <label class="field"><span>Título</span><input ref="titleInput" v-model="title" /></label>
      <fieldset class="modes">
        <legend>Como a IA vai ajudar neste livro</legend>
        <label v-for="m in modes" :key="m.value" class="mode-opt">
          <input v-model="mode" type="radio" name="mode" :value="m.value" />
          <span><strong>{{ m.name }}</strong><br /><small>{{ m.hint }}</small></span>
        </label>
      </fieldset>
      <div class="actions">
        <button class="btn" :disabled="!title.trim()">Criar livro</button>
        <button type="button" class="link" @click="creating = false">Cancelar</button>
      </div>
    </form>

    <div v-else class="actions">
      <button class="btn" @click="startCreating">Novo livro</button>
      <button class="link" @click="lock">Trancar</button>
    </div>
  </main>
</template>

<style scoped>
.books { list-style: none; padding: 0; margin: 0 0 2rem; border-top: 1px solid var(--rule); }
.book {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 1rem;
  width: 100%;
  padding: 1rem 0;
  border: 0;
  border-bottom: 1px solid var(--rule);
  background: none;
  text-align: left;
  cursor: pointer;
}
.book .title { font: 1.3rem/1.3 var(--font-mono); }
.book:hover .title { color: var(--ribbon); }
.book .mode { color: var(--note); font-size: 0.85rem; }
.modes { border: 0; padding: 0; margin: 0 0 1rem; }
.modes legend { color: var(--note); font-size: 0.85rem; margin-bottom: 0.5rem; }
.mode-opt { display: flex; gap: 0.6rem; align-items: flex-start; padding: 0.5rem 0; cursor: pointer; }
.mode-opt input { accent-color: var(--ribbon); margin-top: 0.3rem; }
.mode-opt small { color: var(--note); }
</style>
