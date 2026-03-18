<template>
  <div class="mail-shell-layout">
    <div class="mail-shell-layout__header">
      <AppTitleBar
        :hide-search="hideSearch"
        :search="search"
        :search-placeholder="t('shell.searchPlaceholder')"
        @update:search="$emit('update:search', $event)"
      >
        <template #actions>
          <slot name="actions" />
        </template>
      </AppTitleBar>

      <div
        class="mail-shell-layout__progress"
        :class="{
          'mail-shell-layout__progress--active': loadingActive,
          'mail-shell-layout__progress--indeterminate': loadingActive && loadingProgress === null,
        }"
        :aria-hidden="!loadingActive"
      >
        <div v-if="loadingActive && loadingLabel" class="mail-shell-layout__progress-label">
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

    <main
      class="mail-shell-layout__content"
      :class="{ 'mail-shell-layout__content--dragging': draggingGutter !== null }"
      :style="contentStyle"
    >
      <aside class="mail-shell-layout__sidebar">
        <div class="mail-shell-layout__sidebar-inner">
          <slot name="sidebar" />
        </div>
      </aside>
      <div
        class="mail-shell-layout__gutter"
        :class="{
          'mail-shell-layout__gutter--active': draggingGutter === 'sidebar',
          'mail-shell-layout__gutter--collapsed': sidebarCollapsed,
        }"
        data-gutter="sidebar"
        @mousedown="onGutterDown('sidebar', $event)"
        @dblclick="toggleSidebar"
      >
        <button class="mail-shell-layout__gutter-pill" @mousedown.stop @click="toggleSidebar">
          <v-icon :icon="sidebarCollapsed ? 'mdi-chevron-right' : 'mdi-chevron-left'" size="14" />
        </button>
      </div>
      <section class="mail-shell-layout__list">
        <slot name="list" />
      </section>
      <div
        class="mail-shell-layout__gutter"
        :class="{ 'mail-shell-layout__gutter--active': draggingGutter === 'list' }"
        data-gutter="list"
        @mousedown="onGutterDown('list', $event)"
      />
      <section class="mail-shell-layout__reader">
        <slot name="reader" />
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import AppTitleBar from '@/components/AppTitleBar.vue'

const { t } = useI18n()

withDefaults(
  defineProps<{
    search?: string
    hideSearch?: boolean
    loadingActive?: boolean
    loadingProgress?: number | null
    loadingLabel?: string
  }>(),
  {
    search: '',
    hideSearch: false,
    loadingActive: false,
    loadingProgress: null,
    loadingLabel: '',
  },
)

defineEmits<{
  'update:search': [value: string]
}>()

const sidebarWidth = ref(parseInt(localStorage.getItem('layout-sidebar-w') ?? '260'))
const listWidth = ref(parseInt(localStorage.getItem('layout-list-w') ?? '340'))
const draggingGutter = ref<'sidebar' | 'list' | null>(null)
const sidebarCollapsed = ref(localStorage.getItem('layout-sidebar-collapsed') === 'true')

const toggleSidebar = () => {
  sidebarCollapsed.value = !sidebarCollapsed.value
  localStorage.setItem('layout-sidebar-collapsed', String(sidebarCollapsed.value))
}

const contentStyle = computed(() => ({
  '--sidebar-w': sidebarCollapsed.value ? '0px' : `${sidebarWidth.value}px`,
  '--list-w': `${listWidth.value}px`,
}))

let startPos = 0
let startValue = 0

const onGutterDown = (target: 'sidebar' | 'list', e: MouseEvent) => {
  if (target === 'sidebar' && sidebarCollapsed.value) return
  e.preventDefault()
  draggingGutter.value = target

  startPos = e.clientX
  startValue = target === 'sidebar' ? sidebarWidth.value : listWidth.value

  document.addEventListener('mousemove', onDragMove)
  document.addEventListener('mouseup', onDragEnd)
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
}

const onDragMove = (e: MouseEvent) => {
  const newValue = startValue + (e.clientX - startPos)
  if (draggingGutter.value === 'sidebar') {
    sidebarWidth.value = Math.max(180, Math.min(newValue, 400))
  } else {
    listWidth.value = Math.max(200, Math.min(newValue, 500))
  }
}

const onDragEnd = () => {
  document.removeEventListener('mousemove', onDragMove)
  document.removeEventListener('mouseup', onDragEnd)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''

  localStorage.setItem('layout-sidebar-w', String(sidebarWidth.value))
  localStorage.setItem('layout-list-w', String(listWidth.value))

  draggingGutter.value = null
}

onUnmounted(() => {
  document.removeEventListener('mousemove', onDragMove)
  document.removeEventListener('mouseup', onDragEnd)
})
</script>

<style scoped>
.mail-shell-layout {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  height: 100vh;
  overflow: hidden;
  background: rgb(var(--v-theme-background));
}

.mail-shell-layout__header {
  position: relative;
  z-index: 10;
}

.mail-shell-layout__progress {
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

.mail-shell-layout__progress--active {
  visibility: visible;
  opacity: 1;
  transform: translateY(0);
}

.mail-shell-layout__progress--indeterminate {
  opacity: 1;
}

.mail-shell-layout__progress-label {
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

.mail-shell-layout__progress :deep(.v-progress-linear) {
  width: 100%;
  border-radius: 0;
  background: transparent;
  opacity: 0.88;
}

.mail-shell-layout__progress :deep(.v-progress-linear__background) {
  opacity: 0.14 !important;
}

.mail-shell-layout__progress :deep(.v-progress-linear__determinate),
.mail-shell-layout__progress :deep(.v-progress-linear__indeterminate .long),
.mail-shell-layout__progress :deep(.v-progress-linear__indeterminate .short) {
  border-radius: 0 !important;
}

.mail-shell-layout__content {
  grid-row: 2;
  display: grid;
  grid-template-columns: var(--sidebar-w, 260px) auto var(--list-w, 340px) auto minmax(0, 1fr);
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.mail-shell-layout__content--dragging * {
  pointer-events: none;
}

.mail-shell-layout__sidebar,
.mail-shell-layout__list,
.mail-shell-layout__reader {
  min-height: 0;
  overflow: hidden;
}

.mail-shell-layout__sidebar,
.mail-shell-layout__reader {
  overflow-y: auto;
  overflow-x: hidden;
}

.mail-shell-layout__sidebar {
  position: relative;
  direction: rtl;
}

.mail-shell-layout__sidebar-inner {
  direction: ltr;
  height: 100%;
}

/* ── Gutter (drag handle) ── */

.mail-shell-layout__gutter {
  position: relative;
  width: 1px;
  background: rgba(var(--v-theme-on-surface), 0.06);
  cursor: col-resize;
  z-index: 2;
  transition: background 0.15s;
}

.mail-shell-layout__gutter::after {
  content: '';
  position: absolute;
  inset: 0 -3px;
}

.mail-shell-layout__gutter:hover,
.mail-shell-layout__gutter--active {
  background: rgba(var(--v-theme-primary), 0.4);
}

[data-gutter="sidebar"] {
  background: rgba(var(--v-theme-on-surface), 0.06);
}

[data-gutter="sidebar"]:hover {
  background: rgba(var(--v-theme-primary), 0.4);
}

.mail-shell-layout__gutter--collapsed {
  width: 5px;
  cursor: pointer;
  background: rgba(var(--v-theme-on-surface), 0.08);
}

.mail-shell-layout__gutter--collapsed:hover {
  background: rgba(var(--v-theme-primary), 0.25);
}

.mail-shell-layout__gutter-pill {
  position: absolute;
  top: 50%;
  left: 0;
  translate: 0 -50%;
  width: 20px;
  height: 64px;
  border: none;
  border-radius: 0 8px 8px 0;
  background: rgba(var(--v-theme-on-surface), 0.06);
  color: rgba(var(--v-theme-on-surface), 0.45);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.4;
  transition: opacity 0.2s, background 0.15s, color 0.15s;
  z-index: 3;
}

.mail-shell-layout__gutter:hover .mail-shell-layout__gutter-pill,
.mail-shell-layout__gutter--collapsed .mail-shell-layout__gutter-pill {
  opacity: 1;
}

.mail-shell-layout__gutter-pill:hover {
  background: rgba(var(--v-theme-primary), 0.14);
  color: rgb(var(--v-theme-primary));
}

/* ── ≤840px: tighten defaults ── */

@media (max-width: 840px) {
  .mail-shell-layout__progress {
    left: 0;
    right: 0;
    bottom: 0;
  }

  .mail-shell-layout__progress-label {
    max-width: min(52vw, 220px);
    margin-right: 12px;
  }

  .mail-shell-layout__gutter-pill,
  .mail-shell-layout__gutter {
    display: none;
  }

  .mail-shell-layout__sidebar,
  .mail-shell-layout__list {
    border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.06);
  }
}
</style>
