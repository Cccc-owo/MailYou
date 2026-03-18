<template>
  <v-tooltip v-if="tooltip" :text="tooltip" location="bottom">
    <template #activator="{ props: tip }">
      <v-btn
        v-bind="tip"
        :prepend-icon="prependIcon || undefined"
        :icon="icon ? true : undefined"
        :variant="variant"
        :size="size"
        class="toolbar-action-button"
        rounded="pill"
        :disabled="disabled"
        :loading="loading"
        :aria-label="ariaLabel || tooltip"
        @click="$emit('click')"
      >
        <v-icon v-if="icon" :icon="icon" />
        <slot />
      </v-btn>
    </template>
  </v-tooltip>

  <v-btn
    v-else
    :prepend-icon="prependIcon || undefined"
    :icon="icon ? true : undefined"
    :variant="variant"
    :size="size"
    class="toolbar-action-button"
    rounded="pill"
    :disabled="disabled"
    :loading="loading"
    :aria-label="ariaLabel"
    @click="$emit('click')"
  >
    <v-icon v-if="icon" :icon="icon" />
    <slot />
  </v-btn>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    tooltip?: string
    ariaLabel?: string
    prependIcon?: string
    icon?: string
    variant?: 'flat' | 'text' | 'elevated' | 'outlined' | 'plain' | 'tonal'
    size?: 'x-small' | 'small' | 'default' | 'large' | 'x-large'
    disabled?: boolean
    loading?: boolean
  }>(),
  {
    tooltip: '',
    ariaLabel: '',
    prependIcon: '',
    icon: '',
    variant: 'tonal',
    size: 'small',
    disabled: false,
    loading: false,
  },
)

defineEmits<{
  click: []
}>()
</script>

<style scoped>
.toolbar-action-button {
  min-width: 40px;
}

.toolbar-action-button:deep(.v-btn__content) {
  gap: 8px;
}
</style>
