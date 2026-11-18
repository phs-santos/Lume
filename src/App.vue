<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onMounted, ref } from "vue";
import { canExit } from "./features/editor/exitGuard";
import { profileSelect, profilesList, vaultStatus } from "./lib/api";
import CreateVault from "./features/vault/CreateVault.vue";
import Unlock from "./features/vault/Unlock.vue";
import Recover from "./features/vault/Recover.vue";
import Library from "./features/library/Library.vue";
import MissingVault from "./features/vault/MissingVault.vue";
import EditorScreen from "./features/editor/EditorScreen.vue";

type Screen = "loading" | "missing" | "create" | "unlock" | "recover" | "library" | "editor";
const screen = ref<Screen>("loading");
const bookId = ref("");

async function refresh() {
  const s = await vaultStatus();
  screen.value =
    s === "none" ? "create" : s === "locked" ? "unlock" : s === "missing_vault" ? "missing" : "library";
}

/** Desistiu de criar um usuário novo: volta para o primeiro usuário do computador. */
async function backFromNewProfile() {
  const ps = await profilesList().catch(() => []);
  if (ps[0]) await profileSelect(ps[0].id).catch(() => {});
  await refresh();
}

function openBook(id: string) {
  bookId.value = id;
  screen.value = "editor";
}

onMounted(async () => {
  await refresh();
  // Pedido de saída do app (Cmd+Q): grava o pendente e só então sai.
  await listen("flush-now", async () => {
    if (await canExit()) await invoke("exit_app");
  });
});
</script>

<template>
  <MissingVault v-if="screen === 'missing'" />
  <CreateVault v-else-if="screen === 'create'" @done="screen = 'library'" @back="backFromNewProfile" />
  <Unlock v-else-if="screen === 'unlock'" @done="screen = 'library'" @forgot="screen = 'recover'" @new-profile="screen = 'create'" />
  <Recover v-else-if="screen === 'recover'" @done="screen = 'library'" @back="screen = 'unlock'" />
  <Library v-else-if="screen === 'library'" @open="openBook" @locked="screen = 'unlock'" />
  <EditorScreen v-else-if="screen === 'editor'" :book-id="bookId" @close="screen = 'library'" />
</template>
