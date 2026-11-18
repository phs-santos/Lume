<script setup lang="ts">
import { ref, watchEffect } from "vue";
import QRCode from "qrcode";
import { IconDeviceMobile } from "@tabler/icons-vue";

const props = defineProps<{ url: string; nickname: string }>();
const open = ref(false);
const svg = ref("");

/** Endereço que o celular abre: já vem com o apelido; a senha é digitada lá (nunca vai no QR). */
const link = () => `${props.url.replace(/\/+$/, "")}/?apelido=${encodeURIComponent(props.nickname)}`;

watchEffect(async () => {
  if (!open.value) return;
  svg.value = await QRCode.toString(link(), { type: "svg", margin: 1, color: { dark: "#1c2333", light: "#ffffff" } });
});
</script>

<template>
  <div class="phone">
    <button class="link toggle" :aria-expanded="open" data-test="qr-toggle" @click="open = !open">
      <IconDeviceMobile :size="16" stroke-width="1.75" /> {{ open ? "Esconder" : "Abrir no celular" }}
    </button>
    <div v-if="open" class="qr-box">
      <!-- eslint-disable-next-line vue/no-v-html -- SVG gerado localmente pela biblioteca -->
      <div class="qr" role="img" aria-label="QR code para abrir o Lume no celular" v-html="svg" />
      <p>Aponte a câmera do celular. O Lume abre já com o seu apelido; é só digitar a senha.</p>
      <p class="url">{{ link() }}</p>
    </div>
  </div>
</template>

<style scoped>
.phone { margin: 0 0 0.75rem; }
.toggle { display: inline-flex; align-items: center; gap: 0.35rem; font-size: 0.85rem; }
.qr-box { display: grid; grid-template-columns: 7.5rem 1fr; gap: 0.4rem 0.9rem; align-items: start; margin-top: 0.6rem; padding: 0.75rem; border: 1px solid var(--rule); border-radius: 10px; }
.qr { grid-row: 1 / span 2; width: 7.5rem; height: 7.5rem; }
.qr :deep(svg) { width: 100%; height: 100%; }
.qr-box p { margin: 0; font-size: 0.82rem; color: var(--note); }
.url { font-family: var(--font-mono); font-size: 0.72rem !important; word-break: break-all; }
</style>
