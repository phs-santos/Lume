<script setup lang="ts">
import { ref } from "vue";
import { IconX } from "@tabler/icons-vue";
import { isMac, label, SHORTCUTS, TEXT_SHORTCUTS } from "./shortcuts";

const props = withDefaults(defineProps<{ start?: "guide" | "keys" }>(), { start: "guide" });
const emit = defineEmits<{ close: [] }>();
const mac = isMac();
const groups = ["Escrever", "Navegar", "Aparência", "Painéis"] as const;
const mod = mac ? "⌘" : "Ctrl";
const tab = ref(props.start);
const k = (a: string) => label(SHORTCUTS.find((s) => s.action === a)!, mac);

/** Primeiros passos: o essencial em poucas linhas, na ordem em que a pessoa vai precisar. */
const STEPS = [
  { title: "Escreva", body: "Clique na folha e digite. O Lume salva sozinho a cada pausa; o ponto verde “Salvo” no alto confirma." },
  { title: "Organize em capítulos e cenas", body: "O sumário, à esquerda, mostra o livro. Use o ⋯ de cada linha (ou o clique direito) para criar uma cena antes ou depois, renomear ou apagar. Arraste uma cena para mudar a ordem." },
  { title: "Volte no tempo", body: "Em Versões ficam as versões salvas da cena aberta. Restaurar guarda o texto atual antes, então nada se perde." },
  { title: "Peça ajuda à IA (opcional)", body: "O Assistente, à direita, conversa sobre o livro. A Bíblia guarda personagens, lugares e fatos que a IA encontra no texto. Precisa de uma chave da Inworld; sem ela, o resto do Lume funciona normalmente." },
  { title: "Ideias do celular", body: "Conecte o celular em Ideias para mandar anotações e áudios de qualquer lugar. Elas esperam ali até você decidir o que fazer." },
  { title: "Leve o livro para fora", body: "Exportar gera Word, PDF ou Markdown, do livro inteiro ou só da cena aberta." },
  { title: "Seu texto é só seu", body: "Tudo fica cifrado neste computador. Guarde bem a senha e as 24 palavras de recuperação." },
];
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <section class="sheet-help" role="dialog" aria-labelledby="help-title">
      <header>
        <h2 id="help-title">Ajuda</h2>
        <div class="tabs" role="tablist">
          <button role="tab" :aria-selected="tab === 'guide'" @click="tab = 'guide'">Primeiros passos</button>
          <button role="tab" :aria-selected="tab === 'keys'" @click="tab = 'keys'">Atalhos</button>
        </div>
        <span class="spacer" />
        <button class="link icon" aria-label="Fechar" @click="emit('close')"><IconX :size="18" /></button>
      </header>

      <ol v-if="tab === 'guide'" class="steps">
        <li v-for="s in STEPS" :key="s.title">
          <strong>{{ s.title }}</strong>
          <p>{{ s.body }}</p>
        </li>
        <li class="tip">Atalhos úteis: {{ k("assistant") }} abre o assistente · {{ k("newScene") }} cria uma cena · {{ k("help") }} abre esta ajuda.</li>
      </ol>

      <div v-else class="cols">
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
header { display: flex; align-items: center; gap: 1rem; margin-bottom: 1.25rem; }
h2 { margin: 0; font: 400 1.6rem var(--font-mono); }
.spacer { flex: 1; }
.tabs { display: flex; padding: 2px; border: 1px solid var(--rule); border-radius: 8px; }
.tabs button { border: 0; border-radius: 6px; background: none; padding: 0.25rem 0.7rem; color: var(--note); font-size: 0.85rem; cursor: pointer; }
.tabs button[aria-selected="true"] { background: var(--ink); color: var(--paper); }
.icon { padding: 0 0.45rem; }
.steps { margin: 0; padding: 0 0 0 1.4rem; display: grid; gap: 0.85rem; }
.steps li::marker { color: var(--ribbon); font-weight: 600; }
.steps strong { font-size: 0.95rem; }
.steps p { margin: 0.15rem 0 0; font-size: 0.88rem; line-height: 1.5; color: var(--note); }
.steps .tip { list-style: none; margin-left: -1.4rem; padding-top: 0.6rem; border-top: 1px solid var(--rule); font-size: 0.82rem; color: var(--note); }
.cols { display: grid; grid-template-columns: repeat(auto-fit, minmax(17rem, 1fr)); gap: 1.25rem 2.5rem; }
h3 { font: 500 0.8rem var(--font-ui); color: var(--note); margin: 0 0 0.5rem; letter-spacing: 0; }
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
