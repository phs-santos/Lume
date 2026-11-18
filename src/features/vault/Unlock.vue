<script setup lang="ts">
import VaultLayout from "../../components/VaultLayout.vue";
import PasswordInput from "../../components/PasswordInput.vue";
import { onMounted, ref } from "vue";
import { IconUserPlus, IconUsers } from "@tabler/icons-vue";
import { profileNew, profileSelect, profilesList, vaultNickname, vaultUnlock, type Profile } from "../../lib/api";
import { errorMessage } from "../../lib/errors";

const emit = defineEmits<{ done: []; forgot: []; newProfile: [] }>();
const password = ref("");
const error = ref("");
const busy = ref(false);
const nickname = ref<string | null>(null);

const profiles = ref<Profile[]>([]);
const choosing = ref(false);

async function load() {
  nickname.value = await vaultNickname().catch(() => null);
  try {
    profiles.value = await profilesList();
  } catch {
    profiles.value = [];
  }
}

/** Troca de usuário: o cofre de cada um tem a própria senha. */
async function choose(p: Profile) {
  choosing.value = false;
  if (p.active) return;
  error.value = "";
  password.value = "";
  try {
    await profileSelect(p.id);
  } catch (e) {
    error.value = errorMessage(e);
  }
  await load();
}

async function newProfile() {
  try {
    await profileNew();
    emit("newProfile");
  } catch (e) {
    error.value = errorMessage(e);
  }
}

onMounted(load);

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
      <span class="name"><small>Entrando como</small><strong>{{ nickname }}</strong></span>
      <button type="button" class="link switch" :aria-expanded="choosing" @click="choosing = !choosing"><IconUsers :size="16" /> Trocar</button>
    </div>
    <div v-if="choosing" class="profiles" role="menu" aria-label="Usuários deste computador">
      <button v-for="p in profiles" :key="p.id" type="button" role="menuitemradio" :aria-checked="p.active" class="profile" @click="choose(p)">
        <span class="avatar small" aria-hidden="true">{{ p.nickname.slice(0, 1).toUpperCase() }}</span>
        <span>{{ p.nickname }}</span>
        <small v-if="p.active">atual</small>
      </button>
      <button type="button" class="profile new" @click="newProfile">
        <IconUserPlus :size="18" stroke-width="1.75" /> <span>Novo usuário neste computador</span>
      </button>
    </div>
    <form @submit.prevent="unlock">
      <label class="field"><span>Senha</span><PasswordInput v-model="password" autocomplete="current-password" autofocus /></label>
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
.name { flex: 1; min-width: 0; }
.switch { font-size: 0.82rem; }
.profiles { margin: -1rem 0 1.25rem; padding: 0.35rem; border: 1px solid var(--rule); border-radius: 10px; background: #fff; animation: drop 0.18s ease-out; }
.profile { display: flex; align-items: center; gap: 0.6rem; width: 100%; padding: 0.5rem 0.55rem; border: 0; border-radius: 7px; background: none; color: var(--ink); text-align: left; cursor: pointer; }
.profile:hover { background: color-mix(in srgb, var(--ink) 5%, transparent); }
.profile small { margin-left: auto; color: var(--note); font-size: 0.75rem; }
.profile.new { color: var(--note); border-top: 1px solid var(--rule); border-radius: 0 0 7px 7px; margin-top: 0.25rem; }
.avatar.small { width: 1.75rem; height: 1.75rem; font-size: 0.9rem; }
@keyframes drop { from { opacity: 0; transform: translateY(-4px); } }
.who small { display: block; font-size: 0.75rem; color: var(--note); }
.who strong { font-weight: 500; }
.center { justify-content: center; margin-top: 0.75rem; }
</style>
