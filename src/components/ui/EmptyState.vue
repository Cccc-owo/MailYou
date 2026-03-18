<template>
  <div class="ui-empty-state">
    <v-icon :icon="icon" :size="iconSize" class="ui-empty-state__icon" />
    <div :class="titleClass">{{ title }}</div>
    <div class="ui-empty-state__description">
      <slot>
        {{ description }}
      </slot>
    </div>
    <div v-if="$slots.actions" class="ui-empty-state__actions">
      <slot name="actions" />
    </div>
  </div>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    icon: string
    title: string
    description?: string
    titleClass?: string
    iconSize?: number | string
  }>(),
  {
    description: '',
    titleClass: 'text-h6 mb-1',
    iconSize: 40,
  },
)
</script>
