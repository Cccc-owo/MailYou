<template>
  <div class="app-scrollable-page">
    <div class="app-scrollable-page__header">
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

      <div
        class="app-scrollable-page__progress"
        :class="{
          'app-scrollable-page__progress--active': loadingActive,
          'app-scrollable-page__progress--indeterminate': loadingActive && loadingProgress === null,
        }"
        :aria-hidden="!loadingActive"
      >
        <div v-if="loadingActive && loadingLabel" class="app-scrollable-page__progress-label">
          {{ loadingLabel }}
        </div>
        <v-progress-linear
          :model-value="loadingActive ? loadingProgress ?? undefined : 0"
          :indeterminate="loadingActive && loadingProgress === null"
          color="primary"
          height="4"
          rounded
        />
      </div>
    </div>

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
    loadingActive?: boolean
    loadingProgress?: number | null
    loadingLabel?: string
  }>(),
  {
    title: 'MailYou',
    subtitle: '',
    hideSearch: false,
    search: '',
    searchPlaceholder: '',
    maxWidth: 820,
    contentClass: '',
    loadingActive: false,
    loadingProgress: null,
    loadingLabel: '',
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

.app-scrollable-page__header {
  position: relative;
  z-index: 10;
}

.app-scrollable-page__progress {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
  opacity: 0;
  transform: translateY(-3px);
  pointer-events: none;
  visibility: hidden;
  transition:
    opacity 0.18s ease,
    transform 0.18s ease,
    visibility 0.18s ease;
}

.app-scrollable-page__progress--active {
  visibility: visible;
  opacity: 1;
  transform: translateY(0);
}

.app-scrollable-page__progress--indeterminate {
  opacity: 1;
}

.app-scrollable-page__progress-label {
  width: fit-content;
  max-width: min(40vw, 320px);
  margin-right: 16px;
  padding: 1px 8px;
  border-radius: 999px;
  background: rgba(var(--v-theme-surface), 0.78);
  border: 1px solid rgba(var(--v-theme-on-surface), 0.06);
  font-size: 0.6875rem;
  line-height: 1.25;
  color: rgba(var(--v-theme-on-surface), 0.62);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
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
