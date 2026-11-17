<script setup lang="ts">
// Substituído na Task 6.
defineProps<{ bookId: string }>();
const emit = defineEmits<{ close: [] }>();
</script>

<template>
  <main class="sheet"><button class="link" @click="emit('close')">Voltar aos livros</button></main>
</template>
