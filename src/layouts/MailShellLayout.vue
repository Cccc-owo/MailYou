<template>
  <div class="mail-shell-layout">
    <AppTitleBar :subtitle="subtitle">
      <template #center>
        <v-text-field
          :model-value="search"
          prepend-inner-icon="mdi-magnify"
          hide-details
          density="comfortable"
          placeholder="Search mail, people, and labels"
          @update:model-value="$emit('update:search', $event)"
        />
      </template>

      <template #actions>
        <slot name="actions" />
      </template>
    </AppTitleBar>

    <main class="mail-shell-layout__content">
      <aside class="mail-shell-layout__sidebar">
        <slot name="sidebar" />
      </aside>
      <section class="mail-shell-layout__list">
        <slot name="list" />
      </section>
      <section class="mail-shell-layout__reader">
        <slot name="reader" />
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
import AppTitleBar from '@/components/AppTitleBar.vue'

defineProps<{
  search: string
  subtitle: string
}>()

defineEmits<{
  'update:search': [value: string]
}>()
</script>

<style scoped>
.mail-shell-layout {
  display: grid;
  grid-template-rows: auto 1fr;
  height: 100vh;
  overflow: hidden;
  background: rgb(var(--v-theme-background));
}

.mail-shell-layout__content {
  display: grid;
  grid-template-columns: 280px minmax(320px, 420px) minmax(480px, 1fr);
  min-height: 0;
}

.mail-shell-layout__sidebar,
.mail-shell-layout__list,
.mail-shell-layout__reader {
  min-height: 0;
  overflow: hidden;
}

.mail-shell-layout__sidebar,
.mail-shell-layout__reader {
  overflow: auto;
}

.mail-shell-layout__sidebar,
.mail-shell-layout__list {
  border-right: 1px solid rgba(var(--v-theme-on-surface), 0.06);
}

@media (max-width: 1280px) {
  .mail-shell-layout__content {
    grid-template-columns: 280px minmax(0, 1fr);
    grid-template-areas:
      'sidebar list'
      'reader reader';
  }

  .mail-shell-layout__sidebar {
    grid-area: sidebar;
  }

  .mail-shell-layout__list {
    grid-area: list;
    border-right: none;
  }

  .mail-shell-layout__reader {
    grid-area: reader;
    border-top: 1px solid rgba(var(--v-theme-on-surface), 0.06);
  }
}

@media (max-width: 840px) {
  .mail-shell-layout__content {
    grid-template-columns: minmax(0, 1fr);
    grid-template-areas:
      'sidebar'
      'list'
      'reader';
  }

  .mail-shell-layout__sidebar,
  .mail-shell-layout__list {
    border-right: none;
  }

  .mail-shell-layout__sidebar,
  .mail-shell-layout__list,
  .mail-shell-layout__reader {
    border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.06);
  }

  .mail-shell-layout__reader {
    border-top: none;
    border-bottom: none;
  }
}
</style>
