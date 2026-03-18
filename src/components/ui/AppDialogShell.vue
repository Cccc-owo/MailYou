<template>
  <v-dialog :model-value="modelValue" :max-width="maxWidth" :persistent="persistent" @update:model-value="$emit('update:modelValue', $event)">
    <v-card>
      <v-card-title v-if="title">{{ title }}</v-card-title>
      <slot />
      <v-card-actions v-if="$slots.actions">
        <slot name="actions" />
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: boolean
    title?: string
    maxWidth?: number | string
    persistent?: boolean
  }>(),
  {
    title: '',
    maxWidth: 400,
    persistent: false,
  },
)

defineEmits<{
  'update:modelValue': [value: boolean]
}>()
</script>
