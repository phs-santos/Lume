<script setup lang="ts">
import { computed } from "vue";
import { greeting, timeOfDay } from "../lib/timeOfDay";
import DeskScene from "./DeskScene.vue";

withDefaults(defineProps<{ wide?: boolean }>(), { wide: false });
// Em desenvolvimento, `?hora=22` força o horário da cena para revisar o visual.
const forced = import.meta.env.DEV ? new URLSearchParams(location.search).get("hora") : null;
const time = computed(() => timeOfDay(forced ? Number(forced) : new Date().getHours()));
</script>

<template>
  <div :class="['vault', time]">
    <DeskScene :time="time" class="bg" />
    <main :class="['card', { wide }]">
      <p class="brand">Lume</p>
      <p class="greeting">{{ greeting[time] }}</p>
      <slot />
    </main>
  </div>
</template>

<style scoped>
.vault {
  position: relative;
  height: 100%;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 2rem clamp(1.5rem, 7vw, 7rem);
}
.bg { position: absolute; inset: 0; }
.card {
  position: relative;
  width: min(25rem, 100%);
  max-height: calc(100% - 2rem);
  overflow-y: auto;
  scrollbar-width: none;
  padding: 2.25rem 2.25rem 2rem;
  /* o cartão é sempre papel claro, mesmo com o tema escuro ligado */
  --paper: #f6f7f9; --ink: #1c2333; --note: #646b78; --rule: #cfd4db; --ribbon: #b42318;
  background: var(--paper);
  color: var(--ink);
  border-radius: 14px;
  box-shadow: 0 30px 60px -20px rgba(10, 14, 30, 0.55), 0 0 0 1px rgba(255, 255, 255, 0.06);
  animation: settle 0.5s ease-out both;
}
.card.wide { width: min(36rem, 100%); }
.brand { font: 400 2.4rem/1 var(--font-mono); margin: 0; letter-spacing: -0.02em; }
.card :deep(h1) { font: 500 1.15rem/1.3 var(--font-ui); margin: 0 0 0.4rem; }
.card :deep(.lede) { color: var(--note); margin: 0 0 1.5rem; font-size: 0.95rem; }
.greeting { font: italic 400 1.05rem/1.4 var(--font-serif); color: #646b78; margin: 0.5rem 0 1.75rem; }
@keyframes settle { from { opacity: 0; transform: translateY(12px); } to { opacity: 1; transform: none; } }
@media (max-width: 760px) {
  .vault { justify-content: center; }
}
@media (prefers-reduced-motion: reduce) { .card { animation: none; } }
</style>
