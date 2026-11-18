<script setup lang="ts">
import { IconX } from "@tabler/icons-vue";
import { onBeforeUnmount, onMounted, ref } from "vue";
import { bookMetaGet, bookMetaSet, imageAdd } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import { COVER_SIDE, prepareImage, toBase64 } from "../editor/imageFile";
import { imageUrl } from "../editor/imageUrl";
import { generateCover } from "./cover";

const props = defineProps<{ bookId: string; title: string; color: string }>();
const emit = defineEmits<{ close: []; saved: [] }>();

const author = ref("");
const synopsis = ref("");
const dedication = ref("");
const coverId = ref<string | null>(null);
const backId = ref<string | null>(null);
const coverPreview = ref<string | null>(null);
const backPreview = ref<string | null>(null);
const error = ref("");
const busy = ref(false);
/** Só deixa salvar depois de carregar a ficha (senão gravaria tudo vazio por cima). */
const loaded = ref(false);
let generatedUrl: string | null = null;
let redraw: ReturnType<typeof setTimeout> | null = null;
/** A capa gerada acompanha o nome do autor enquanto ela digita. */
function authorTyped() {
  if (coverId.value) return;
  if (redraw) clearTimeout(redraw);
  redraw = setTimeout(showGenerated, 400);
}

async function showGenerated() {
  try {
    const png = await generateCover({ title: props.title, author: author.value, color: props.color });
    if (generatedUrl) URL.revokeObjectURL(generatedUrl);
    generatedUrl = URL.createObjectURL(new Blob([png as Uint8Array<ArrayBuffer>], { type: "image/png" }));
    if (!coverId.value) coverPreview.value = generatedUrl;
  } catch {
    /* sem canvas (testes): só não há prévia */
  }
}

onMounted(async () => {
  let m;
  try {
    m = await bookMetaGet(props.bookId);
  } catch (e) {
    error.value = `Não consegui abrir a ficha: ${errorMessage(e)}`;
    return;
  }
  author.value = m.author ?? "";
  synopsis.value = m.synopsis ?? "";
  dedication.value = m.dedication ?? "";
  coverId.value = m.cover_image_id;
  backId.value = m.back_image_id;
  if (coverId.value) coverPreview.value = await imageUrl(coverId.value).catch(() => null);
  if (backId.value) backPreview.value = await imageUrl(backId.value).catch(() => null);
  loaded.value = true;
  await showGenerated();
});
onBeforeUnmount(() => {
  if (redraw) clearTimeout(redraw);
  if (generatedUrl) URL.revokeObjectURL(generatedUrl);
});

async function pick(which: "cover" | "back", e: Event) {
  const f = (e.target as HTMLInputElement).files?.[0];
  (e.target as HTMLInputElement).value = "";
  if (!f) return;
  error.value = "";
  try {
    const img = await prepareImage(f, COVER_SIDE);
    const id = await imageAdd(props.bookId, img.mime, toBase64(img.bytes), img.width, img.height);
    const url = await imageUrl(id);
    if (which === "cover") [coverId.value, coverPreview.value] = [id, url];
    else [backId.value, backPreview.value] = [id, url];
  } catch (err) {
    error.value = errorMessage(err);
  }
}

function useGenerated() {
  coverId.value = null;
  coverPreview.value = generatedUrl;
}

async function save() {
  if (!loaded.value) return;
  busy.value = true;
  error.value = "";
  try {
    await bookMetaSet(props.bookId, {
      author: author.value.trim() || null,
      synopsis: synopsis.value.trim() || null,
      dedication: dedication.value.trim() || null,
      cover_image_id: coverId.value,
      back_image_id: backId.value,
    });
    emit("saved");
    emit("close");
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
</script>

<!-- Ficha do livro: o que vai na capa, no rosto, na dedicatória e na contracapa do EPUB. -->
<template>
  <div class="backdrop" @click.self="emit('close')">
    <section class="sheet-book" role="dialog" aria-labelledby="book-sheet-title" @keydown.esc="emit('close')">
      <header>
        <h2 id="book-sheet-title">Ficha do livro</h2>
        <button class="link icon" aria-label="Fechar" @click="emit('close')"><IconX :size="18" /></button>
      </header>
      <div class="cols">
        <div class="fields">
          <label class="field"><span>Autor</span><input v-model="author" data-test="author" placeholder="Como o nome aparece na capa" @input="authorTyped" /></label>
          <label class="field"><span>Sinopse (vai na contracapa)</span><textarea v-model="synopsis" data-test="synopsis" rows="5" /></label>
          <label class="field"><span>Dedicatória (opcional)</span><textarea v-model="dedication" data-test="dedication" rows="2" /></label>
        </div>
        <div class="covers">
          <div class="cover-box">
            <span class="lbl">Capa</span>
            <img v-if="coverPreview" :src="coverPreview" alt="Prévia da capa" class="preview" />
            <div v-else class="preview empty">Capa gerada com título e autor</div>
            <div class="row">
              <label class="link">Minha imagem<input type="file" accept="image/*" hidden @change="pick('cover', $event)" /></label>
              <button v-if="coverId" type="button" class="link" @click="useGenerated">Usar a gerada</button>
            </div>
          </div>
          <div class="cover-box">
            <span class="lbl">Contracapa (opcional)</span>
            <img v-if="backPreview" :src="backPreview" alt="Prévia da contracapa" class="preview" />
            <div v-else class="preview empty">Só a sinopse</div>
            <div class="row">
              <label class="link">Escolher imagem<input type="file" accept="image/*" hidden @change="pick('back', $event)" /></label>
              <button v-if="backId" type="button" class="link" @click="(backId = null), (backPreview = null)">Tirar</button>
            </div>
          </div>
        </div>
      </div>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
      <div class="actions end">
        <button class="btn" data-test="save" :disabled="busy || !loaded" @click="save">Salvar ficha</button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.backdrop { position: fixed; inset: 0; display: grid; place-items: center; padding: 1rem; background: rgba(15, 20, 38, 0.45); z-index: 30; }
.sheet-book { width: min(46rem, 100%); max-height: 92vh; overflow-y: auto; padding: 1.4rem 1.6rem; background: var(--paper); color: var(--ink); border-radius: 14px; box-shadow: 0 30px 60px -20px rgba(10, 14, 30, 0.5); }
header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
h2 { margin: 0; font: 400 1.5rem var(--font-mono); }
.icon { padding: 0 0.45rem; }
.cols { display: grid; grid-template-columns: minmax(0, 1fr) 15rem; gap: 1.5rem; }
.covers { display: grid; gap: 1rem; }
.cover-box { display: flex; flex-direction: column; gap: 0.4rem; }
.lbl { font-size: 0.85rem; color: var(--note); }
.preview { width: 100%; aspect-ratio: 5 / 8; object-fit: cover; border-radius: 3px 6px 6px 3px; box-shadow: 0 8px 18px -8px rgba(0, 0, 0, 0.45); }
.preview.empty { display: grid; place-items: center; padding: 1rem; border: 1.5px dashed var(--rule); color: var(--note); font-size: 0.82rem; text-align: center; box-shadow: none; }
.row { display: flex; flex-wrap: wrap; gap: 0.25rem; margin: 0 -0.5rem; font-size: 0.85rem; }
.row label { cursor: pointer; }
.end { justify-content: flex-end; }
@media (max-width: 640px) { .cols { grid-template-columns: 1fr; } }
</style>
