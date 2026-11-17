<script setup lang="ts">
import VaultLayout from "../../components/VaultLayout.vue";
</script>

<template>
  <VaultLayout>
    <h1>O cofre sumiu</h1>
    <p class="lede">
      Seu livro cifrado continua neste computador, mas o arquivo que guarda a chave dele (<code>vault.json</code>) não
      está mais na pasta do app. Sem ele, o livro não abre, e o Lume não vai criar um cofre novo por cima.
    </p>
    <p class="lede">
      Na pasta de dados do Lume existe uma cópia de segurança chamada <code>vault.json.bak</code>. Feche o app, renomeie
      essa cópia para <code>vault.json</code> e abra o Lume de novo.
    </p>
  </VaultLayout>
</template>

<style scoped>
code { font-family: var(--font-mono); font-size: 0.95em; }
</style>
