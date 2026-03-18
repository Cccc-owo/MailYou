<template>
  <div class="ui-section-header">
    <v-icon v-if="icon" :icon="icon" :size="iconSize" />
    <span class="ui-section-header__title">{{ title }}</span>
    <v-spacer />
    <slot />
  </div>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    title: string
    icon?: string
    iconSize?: number | string
  }>(),
  {
    icon: undefined,
    iconSize: 18,
  },
)
</script>
