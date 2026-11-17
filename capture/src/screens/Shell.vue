<script setup lang="ts">
import DeskScene from "../../../src/components/DeskScene.vue";
import { greeting, timeOfDay } from "../../../src/lib/timeOfDay";

const time = timeOfDay(new Date().getHours());
</script>

<!-- Moldura das telas do celular: cena da escrivaninha em cima, cartão de papel embaixo. -->
<template>
  <div class="shell">
    <div class="scene"><DeskScene :time="time" crop="middle" /></div>
    <main class="card">
      <p class="brand">Lume</p>
      <p class="greeting">{{ greeting[time] }}</p>
      <slot />
    </main>
  </div>
</template>

<style scoped>
.shell { min-height: 100%; display: flex; flex-direction: column; }
.scene { height: 34vh; min-height: 11rem; flex-shrink: 0; }
.card {
  flex: 1;
  margin-top: -1.5rem;
  padding: 1.75rem 1.5rem calc(1.5rem + env(safe-area-inset-bottom));
  background: var(--paper);
  border-radius: 22px 22px 0 0;
  box-shadow: 0 -12px 30px rgba(10, 14, 30, 0.35);
  position: relative;
}
.brand { font: 400 2rem/1 var(--font-mono); margin: 0; letter-spacing: -0.02em; }
.greeting { font: italic 1rem/1.4 var(--font-serif); color: var(--note); margin: 0.4rem 0 1.5rem; }
</style>
