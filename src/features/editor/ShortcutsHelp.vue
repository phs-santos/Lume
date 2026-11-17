<script setup lang="ts">
import { IconX } from "@tabler/icons-vue";
import { isMac, label, SHORTCUTS, TEXT_SHORTCUTS } from "./shortcuts";

const emit = defineEmits<{ close: [] }>();
const mac = isMac();
const groups = ["Escrever", "Navegar", "Aparência", "Painéis"] as const;
const mod = mac ? "⌘" : "Ctrl";
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <section class="sheet-help" role="dialog" aria-labelledby="help-title">
      <header>
        <h2 id="help-title">Atalhos</h2>
        <button class="link icon" aria-label="Fechar" @click="emit('close')"><IconX :size="18" /></button>
      </header>
      <div class="cols">
        <div v-for="g in groups" :key="g" class="group">
          <h3>{{ g }}</h3>
          <dl>
            <template v-for="s in SHORTCUTS.filter((x) => x.group === g)" :key="s.action">
              <dt>{{ s.description }}</dt>
              <dd><kbd v-for="p in label(s, mac).split(' ')" :key="p">{{ p }}</kbd></dd>
            </template>
            <template v-if="g === 'Escrever'">
              <template v-for="t in TEXT_SHORTCUTS" :key="t.description">
                <dt>{{ t.description }}</dt>
                <dd><kbd>{{ mod }}</kbd><kbd>{{ t.keys[0].toUpperCase() }}</kbd></dd>
              </template>
            </template>
          </dl>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.backdrop { position: fixed; inset: 0; display: grid; place-items: center; padding: 1rem; background: rgba(15, 20, 38, 0.45); z-index: 30; }
.sheet-help {
  width: min(44rem, 100%);
  max-height: 90vh;
  overflow-y: auto;
  scrollbar-width: none;
  padding: 1.5rem 1.75rem 1.75rem;
  background: var(--paper);
  color: var(--ink);
  border-radius: 14px;
  box-shadow: 0 30px 60px -20px rgba(10, 14, 30, 0.5);
}
header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
h2 { margin: 0; font: 400 1.6rem var(--font-mono); }
.icon { padding: 0 0.45rem; }
.cols { display: grid; grid-template-columns: repeat(auto-fit, minmax(17rem, 1fr)); gap: 1.25rem 2.5rem; }
h3 { font-size: 0.8rem; font-weight: 500; color: var(--note); margin: 0 0 0.5rem; }
dl { display: grid; grid-template-columns: 1fr auto; gap: 0.45rem 1rem; margin: 0; align-items: center; }
dt { font-size: 0.9rem; }
dd { margin: 0; display: flex; gap: 0.25rem; }
kbd {
  min-width: 1.6rem;
  padding: 0.1rem 0.4rem;
  border: 1px solid var(--rule);
  border-bottom-width: 2px;
  border-radius: 6px;
  background: var(--sheet);
  font: 500 0.78rem var(--font-ui);
  text-align: center;
}
</style>
