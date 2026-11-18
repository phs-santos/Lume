<script setup lang="ts">
import { NodeViewWrapper, nodeViewProps } from "@tiptap/vue-3";
import { onMounted, ref } from "vue";
import { imageId } from "./figure";
import { imageUrl } from "./imageUrl";

const props = defineProps(nodeViewProps);
const url = ref<string | null>(null);
const failed = ref(false);

onMounted(async () => {
  const id = imageId(props.node.attrs.src);
  if (!id) return void (failed.value = true);
  try {
    url.value = await imageUrl(id);
  } catch {
    failed.value = true;
  }
});

function setCaption(e: Event) {
  props.updateAttributes({ caption: (e.target as HTMLInputElement).value });
}

/** Teclas da legenda não vão para o editor; atalhos com ⌘/Ctrl (salvar, ajuda…) continuam valendo. */
function captionKey(e: KeyboardEvent) {
  if (!e.metaKey && !e.ctrlKey) e.stopPropagation();
}
</script>

<!-- Imagem no meio do texto, com legenda que se edita direto embaixo dela. -->
<template>
  <NodeViewWrapper as="figure" :class="['lume-figure', { selected }]" data-drag-handle>
    <img v-if="url" :src="url" :width="node.attrs.width ?? undefined" :height="node.attrs.height ?? undefined" alt="" draggable="false" />
    <div v-else class="missing">{{ failed ? "Imagem não encontrada" : "Carregando imagem…" }}</div>
    <input
      class="caption"
      :value="node.attrs.caption"
      placeholder="Legenda (opcional)"
      aria-label="Legenda da imagem"
      spellcheck="true"
      @input="setCaption"
      @keydown="captionKey"
    />
  </NodeViewWrapper>
</template>

<style scoped>
.lume-figure { margin: 1.4em 0; text-align: center; text-indent: 0; }
.lume-figure img { max-width: 100%; height: auto; border-radius: 2px; box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15); }
.lume-figure.selected img { outline: 2px solid var(--ribbon); outline-offset: 3px; }
.missing { padding: 2rem; border: 1px dashed var(--rule); color: var(--note); font-size: 0.85rem; }
.caption {
  display: block;
  width: 80%;
  margin: 0.5rem auto 0;
  border: 0;
  background: none;
  text-align: center;
  font: italic 0.92em/1.4 var(--font-serif);
  color: var(--note);
}
.caption:focus { outline: none; color: var(--ink); }
.caption::placeholder { color: color-mix(in srgb, var(--note) 55%, transparent); }
</style>
