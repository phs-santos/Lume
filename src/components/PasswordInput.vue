<script setup lang="ts">
import { IconEye, IconEyeOff } from "@tabler/icons-vue";
import { ref } from "vue";

defineOptions({ inheritAttrs: false }); // aria-* e afins vão para o campo, não para o envoltório
defineProps<{ modelValue: string; autocomplete?: string; autofocus?: boolean }>();
const emit = defineEmits<{ "update:modelValue": [v: string] }>();
const visible = ref(false);
</script>

<!-- Campo de senha com o botão de olho para conferir o que foi digitado. -->
<template>
  <span class="pw">
    <input
      :type="visible ? 'text' : 'password'"
      :value="modelValue"
      :autocomplete="autocomplete"
      :autofocus="autofocus"
      v-bind="$attrs"
      spellcheck="false"
      autocapitalize="off"
      @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    />
    <button type="button" class="eye" :aria-label="visible ? 'Esconder senha' : 'Mostrar senha'" :title="visible ? 'Esconder senha' : 'Mostrar senha'" @click="visible = !visible">
      <IconEyeOff v-if="visible" :size="18" stroke-width="1.75" />
      <IconEye v-else :size="18" stroke-width="1.75" />
    </button>
  </span>
</template>

<style scoped>
.pw { position: relative; display: block; }
.pw input { padding-right: 2.75rem; }
.eye {
  position: absolute;
  top: 50%;
  right: 0.4rem;
  transform: translateY(-50%);
  display: grid;
  place-items: center;
  width: 2rem;
  height: 2rem;
  border: 0;
  border-radius: 6px;
  background: none;
  color: var(--note);
  cursor: pointer;
}
.eye:hover { color: var(--ink); background: color-mix(in srgb, var(--ink) 6%, transparent); }
</style>
