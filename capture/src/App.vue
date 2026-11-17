<script setup lang="ts">
import Capture from "./screens/Capture.vue";
import Login from "./screens/Login.vue";
import { loadSession, session } from "./session";

loadSession();
</script>

<template>
  <Capture v-if="session.token && session.publicKey" />
  <Login v-else />
</template>
