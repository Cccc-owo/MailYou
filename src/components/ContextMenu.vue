<template>
  <v-menu
    :model-value="modelValue"
    :target="[x, y]"
    min-width="200"
    @update:model-value="$emit('update:modelValue', $event)"
  >
    <v-list density="compact">
      <slot />
    </v-list>
  </v-menu>
</template>

<script setup lang="ts">
defineProps<{
  modelValue: boolean
  x: number
  y: number
}>()

defineEmits<{
  'update:modelValue': [value: boolean]
}>()
</script>
