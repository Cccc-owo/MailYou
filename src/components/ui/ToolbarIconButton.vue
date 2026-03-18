<template>
  <v-tooltip v-if="tooltip" :text="tooltip" location="bottom">
    <template #activator="{ props: tip }">
      <v-btn
        v-bind="mergeButtonAttrs(tip)"
        icon
        :variant="variant"
        :size="size"
        :aria-label="ariaLabel || tooltip"
        @click="$emit('click')"
      >
        <v-icon :icon="icon" />
      </v-btn>
    </template>
  </v-tooltip>

  <v-btn
    v-else
    v-bind="$attrs"
    icon
    :variant="variant"
    :size="size"
    :aria-label="ariaLabel"
    @click="$emit('click')"
  >
    <v-icon :icon="icon" />
  </v-btn>
</template>

<script setup lang="ts">
import { useAttrs } from 'vue'

defineOptions({
  inheritAttrs: false,
})

const attrs = useAttrs()

withDefaults(
  defineProps<{
    icon: string
    tooltip?: string
    ariaLabel?: string
    variant?: 'flat' | 'text' | 'elevated' | 'outlined' | 'plain' | 'tonal'
    size?: 'x-small' | 'small' | 'default' | 'large' | 'x-large'
  }>(),
  {
    tooltip: '',
    ariaLabel: '',
    variant: 'text',
    size: 'small',
  },
)

defineEmits<{
  click: []
}>()

const mergeButtonAttrs = (tip: Record<string, unknown>) => ({
  ...attrs,
  ...tip,
})
</script>
