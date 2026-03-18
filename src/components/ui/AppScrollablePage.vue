<template>
  <div class="app-scrollable-page">
    <AppTitleBar
      :title="title"
      :subtitle="subtitle"
      :hide-search="hideSearch"
      :search="search"
      :search-placeholder="searchPlaceholder"
      @update:search="$emit('update:search', $event)"
    >
      <template #actions>
        <slot name="actions" />
      </template>
    </AppTitleBar>

    <div class="app-scrollable-page__scroller">
      <v-container :class="['app-scrollable-page__content', contentClass]" :max-width="maxWidth">
        <slot />
      </v-container>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppTitleBar from '@/components/AppTitleBar.vue'

withDefaults(
  defineProps<{
    title?: string
    subtitle?: string
    hideSearch?: boolean
    search?: string
    searchPlaceholder?: string
    maxWidth?: number | string
    contentClass?: string
  }>(),
  {
    title: 'MailYou',
    subtitle: '',
    hideSearch: false,
    search: '',
    searchPlaceholder: '',
    maxWidth: 820,
    contentClass: '',
  },
)

defineEmits<{
  'update:search': [value: string]
}>()
</script>

<style scoped>
.app-scrollable-page {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  height: 100vh;
  background: rgb(var(--v-theme-background));
  overflow: hidden;
}

.app-scrollable-page__scroller {
  min-height: 0;
  overflow-y: auto;
}

.app-scrollable-page__content {
  padding-block: 32px 48px;
}

@media (max-width: 600px) {
  .app-scrollable-page__content {
    padding-block: 16px 32px;
  }
}
</style>
