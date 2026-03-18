<template>
  <v-tooltip v-if="iconOnly" :text="label" location="bottom">
    <template #activator="{ props: tip }">
      <v-btn v-bind="tip" class="back-action-button" icon rounded="pill" variant="tonal" @click="$emit('click')">
        <v-icon :icon="icon" />
      </v-btn>
    </template>
  </v-tooltip>

  <v-btn v-else class="back-action-button" :prepend-icon="icon" rounded="pill" variant="tonal" @click="$emit('click')">
    {{ label }}
  </v-btn>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string
    icon?: string
    iconOnly?: boolean
  }>(),
  {
    icon: 'mdi-arrow-left',
    iconOnly: false,
  },
)

defineEmits<{
  click: []
}>()
</script>
