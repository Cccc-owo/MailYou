<template>
  <section class="form-section-card">
    <SectionHeader class="form-section-card__header" :title="title" :icon="icon" />
    <div class="form-section-card__body ui-surface-panel">
      <slot />
    </div>
  </section>
</template>

<script setup lang="ts">
import SectionHeader from '@/components/ui/SectionHeader.vue'

defineProps<{
  title: string
  icon?: string
}>()
</script>

<style scoped>
.form-section-card {
  display: flex;
  flex-direction: column;
}

.form-section-card + .form-section-card {
  margin-top: 8px;
}

.form-section-card__header {
  padding: 0 16px;
  margin-bottom: 12px;
  letter-spacing: 0.1em;
}
</style>
