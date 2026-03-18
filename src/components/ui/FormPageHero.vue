<template>
  <div class="form-page-hero">
    <div class="text-overline mb-2">{{ eyebrow }}</div>
    <div class="text-h4 mb-6">{{ title }}</div>
    <div v-if="description" class="text-body-2 text-medium-emphasis mb-6">
      {{ description }}
    </div>
  </div>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    eyebrow: string
    title: string
    description?: string
  }>(),
  {
    description: '',
  },
)
</script>
