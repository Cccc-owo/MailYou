<template>
  <div class="snackbar-actions ui-inline-actions">
    <v-btn
      v-for="action in actions"
      :key="action.key"
      variant="text"
      :size="action.size ?? 'small'"
      :icon="action.icon"
      @click="$emit('select', action.key)"
    >
      <template v-if="!action.icon">{{ action.label }}</template>
    </v-btn>
  </div>
</template>

<script setup lang="ts">
export type SnackbarActionItem = {
  key: string
  label?: string
  icon?: string
  size?: 'x-small' | 'small' | 'default' | 'large' | 'x-large'
}

defineProps<{
  actions: SnackbarActionItem[]
}>()

defineEmits<{
  select: [key: string]
}>()
</script>

<style scoped>
.snackbar-actions {
  gap: 0;
}
</style>
