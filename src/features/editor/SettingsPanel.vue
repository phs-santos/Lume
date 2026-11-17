<script setup lang="ts">
import { prefs, setPref } from "./settings";

const emit = defineEmits<{ close: [] }>();
</script>

<template>
  <aside class="panel" aria-label="Ajustes">
    <div class="row">
      <span>Letra</span>
      <div class="seg">
        <button :aria-pressed="prefs.font === 'serif'" class="serif" @click="setPref('font', 'serif')">Livro</button>
        <button :aria-pressed="prefs.font === 'mono'" class="mono" @click="setPref('font', 'mono')">Máquina</button>
      </div>
    </div>
    <div class="row">
      <span>Papel</span>
      <div class="seg">
        <button :aria-pressed="prefs.theme === 'papel'" @click="setPref('theme', 'papel')">Claro</button>
        <button :aria-pressed="prefs.theme === 'escuro'" @click="setPref('theme', 'escuro')">Escuro</button>
      </div>
    </div>
    <label class="row">
      <span>Esmaecer o resto do texto</span>
      <input type="checkbox" :checked="prefs.focus === 'on'" @change="setPref('focus', prefs.focus === 'on' ? 'off' : 'on')" />
    </label>
    <label class="row">
      <span>Mostrar a máquina</span>
      <input type="checkbox" :checked="prefs.machine === 'on'" @change="setPref('machine', prefs.machine === 'on' ? 'off' : 'on')" />
    </label>
    <label class="row">
      <span>Som de tecla</span>
      <input type="checkbox" :checked="prefs.keysound === 'on'" @change="setPref('keysound', prefs.keysound === 'on' ? 'off' : 'on')" />
    </label>
    <button class="link" @click="emit('close')">Fechar</button>
  </aside>
</template>

<style scoped>
.panel {
  position: absolute;
  top: 3.4rem;
  right: 1rem;
  width: 18rem;
  padding: 1rem 1.25rem;
  background: var(--paper);
  border: 1px solid var(--rule);
  border-radius: 12px;
  box-shadow: 0 18px 40px -16px rgba(28, 35, 51, 0.35);
  z-index: 10;
  font-size: 0.9rem;
}
.row { display: flex; justify-content: space-between; align-items: center; gap: 1rem; padding: 0.45rem 0; }
.row input { accent-color: var(--ribbon); width: 1rem; height: 1rem; }
.seg { display: flex; padding: 2px; border: 1px solid var(--rule); border-radius: 8px; }
.seg button { border: 0; border-radius: 6px; background: none; padding: 0.25rem 0.65rem; cursor: pointer; color: var(--note); }
.seg button[aria-pressed="true"] { background: var(--ink); color: var(--paper); }
.seg .serif { font-family: var(--font-serif); }
.seg .mono { font-family: var(--font-mono); }
.link { margin-top: 0.5rem; }
</style>
