<script setup lang="ts">
import { onMounted, ref } from "vue";
import { vaultStatus } from "./lib/api";
import CreateVault from "./features/vault/CreateVault.vue";
import Unlock from "./features/vault/Unlock.vue";
import Recover from "./features/vault/Recover.vue";
import Library from "./features/library/Library.vue";
import EditorScreen from "./features/editor/EditorScreen.vue";

type Screen = "loading" | "create" | "unlock" | "recover" | "library" | "editor";
const screen = ref<Screen>("loading");
const bookId = ref("");

async function refresh() {
  const s = await vaultStatus();
  screen.value = s === "none" ? "create" : s === "locked" ? "unlock" : "library";
}

function openBook(id: string) {
  bookId.value = id;
  screen.value = "editor";
}

onMounted(refresh);
</script>

<template>
  <CreateVault v-if="screen === 'create'" @done="screen = 'library'" />
  <Unlock v-else-if="screen === 'unlock'" @done="screen = 'library'" @forgot="screen = 'recover'" />
  <Recover v-else-if="screen === 'recover'" @done="screen = 'library'" @back="screen = 'unlock'" />
  <Library v-else-if="screen === 'library'" @open="openBook" @locked="screen = 'unlock'" />
  <EditorScreen v-else-if="screen === 'editor'" :book-id="bookId" @close="screen = 'library'" />
</template>
