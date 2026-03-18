<template>
  <header class="app-title-bar">
    <div class="app-title-bar__brand">
      <div class="app-title-bar__logo">
        <svg viewBox="0 0 512 512" width="20" height="20" fill="none" stroke="currentColor" stroke-width="32" stroke-linecap="round" stroke-linejoin="round">
          <rect x="72" y="148" width="368" height="272" rx="24"/>
          <polyline points="96,148 256,332 416,148"/>
        </svg>
      </div>
      <div class="app-title-bar__copy">
        <div class="text-h1">{{ title }}</div>
        <div v-if="subtitle" class="app-title-bar__subtitle">{{ subtitle }}</div>
      </div>
    </div>

    <div class="app-title-bar__actions ui-inline-actions">
      <div v-if="!hideSearch" class="app-title-bar__search" :class="{ 'app-title-bar__search--open': searchOpen }">
        <v-text-field
          v-if="searchOpen"
          ref="searchFieldRef"
          :model-value="search"
          prepend-inner-icon="mdi-magnify"
          append-inner-icon="mdi-close"
          hide-details
          density="compact"
          :placeholder="searchPlaceholder"
          class="app-title-bar__search-field"
          @update:model-value="$emit('update:search', $event)"
          @click:append-inner="closeSearch"
          @keydown.escape="closeSearch"
          @contextmenu="openSearchMenu"
        />
        <v-tooltip v-else :text="searchPlaceholder" location="bottom">
          <template #activator="{ props: tip }">
            <v-btn
              v-bind="tip"
              icon
              variant="tonal"
              size="small"
              class="app-title-bar__action-btn"
              :aria-label="searchPlaceholder"
              @click="openSearch"
            >
              <svg class="app-title-bar__icon" viewBox="0 0 24 24" aria-hidden="true">
                <path :d="mdiMagnify" />
              </svg>
            </v-btn>
          </template>
        </v-tooltip>
      </div>
      <slot name="actions" />
    </div>

    <ContextMenu v-model="searchCtx.isOpen.value" :x="searchCtx.x.value" :y="searchCtx.y.value">
      <v-list-item v-if="searchHasSelection" prepend-icon="mdi-content-copy" :title="t('reader.copy')" @click="searchCopy" />
      <v-list-item prepend-icon="mdi-content-paste" :title="t('reader.paste')" @click="searchPaste" />
      <v-divider />
      <v-list-item prepend-icon="mdi-select-all" :title="t('reader.selectAll')" @click="searchSelectAll" />
    </ContextMenu>

    <div v-if="isSupported" class="app-title-bar__window-controls ui-inline-actions">
      <v-tooltip :text="t('titleBar.minimize')" location="bottom">
        <template #activator="{ props: tip }">
          <v-btn
            v-bind="tip"
            icon
            variant="text"
            size="small"
            :aria-label="t('titleBar.minimize')"
            @click="minimize"
          >
            <svg class="app-title-bar__icon" viewBox="0 0 24 24" aria-hidden="true">
              <path :d="mdiWindowMinimize" />
            </svg>
          </v-btn>
        </template>
      </v-tooltip>
      <v-tooltip :text="isMaximized ? t('titleBar.restore') : t('titleBar.maximize')" location="bottom">
        <template #activator="{ props: tip }">
          <v-btn
            v-bind="tip"
            icon
            variant="text"
            size="small"
            :aria-label="isMaximized ? t('titleBar.restore') : t('titleBar.maximize')"
            @click="toggleMaximize"
          >
            <svg class="app-title-bar__icon" viewBox="0 0 24 24" aria-hidden="true">
              <path :d="isMaximized ? mdiWindowRestore : mdiWindowMaximize" />
            </svg>
          </v-btn>
        </template>
      </v-tooltip>
      <v-tooltip :text="t('titleBar.close')" location="bottom">
        <template #activator="{ props: tip }">
          <v-btn
            v-bind="tip"
            class="app-title-bar__close"
            icon
            variant="text"
            size="small"
            :aria-label="t('titleBar.close')"
            @click="close"
          >
            <svg class="app-title-bar__icon" viewBox="0 0 24 24" aria-hidden="true">
              <path :d="mdiClose" />
            </svg>
          </v-btn>
        </template>
      </v-tooltip>
    </div>
  </header>
</template>

<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { mdiClose, mdiMagnify, mdiWindowMaximize, mdiWindowMinimize, mdiWindowRestore } from '@mdi/js'
import { useI18n } from 'vue-i18n'
import { useWindowControls } from '@/composables/useWindowControls'
import { useContextMenu } from '@/composables/useContextMenu'
import ContextMenu from '@/components/ContextMenu.vue'

const { t } = useI18n()

withDefaults(
  defineProps<{
    hideSearch?: boolean
    search?: string
    searchPlaceholder?: string
    title?: string
    subtitle?: string
  }>(),
  {
    hideSearch: false,
    search: '',
    searchPlaceholder: '',
    title: 'MailYou',
    subtitle: '',
  },
)

const emit = defineEmits<{
  'update:search': [value: string]
}>()

const { close, isMaximized, isSupported, minimize, toggleMaximize } = useWindowControls()

const searchOpen = ref(false)
const searchFieldRef = ref<{ focus: () => void } | null>(null)

const openSearch = async () => {
  searchOpen.value = true
  await nextTick()
  searchFieldRef.value?.focus()
}

const closeSearch = () => {
  searchOpen.value = false
}

const searchCtx = useContextMenu()
const searchHasSelection = ref(false)

const getSearchInput = (): HTMLInputElement | null => {
  const el = searchFieldRef.value as unknown as { $el?: HTMLElement } | null
  return el?.$el?.querySelector('input') ?? null
}

const openSearchMenu = (e: MouseEvent) => {
  searchHasSelection.value = Boolean(window.getSelection()?.toString())
  searchCtx.open(e)
}

const searchCopy = () => {
  const input = getSearchInput()
  if (!input) return
  const selected = input.value.substring(input.selectionStart ?? 0, input.selectionEnd ?? 0)
  if (selected) navigator.clipboard.writeText(selected)
}

const searchPaste = async () => {
  const input = getSearchInput()
  if (!input) return
  const text = await navigator.clipboard.readText()
  const start = input.selectionStart ?? input.value.length
  const end = input.selectionEnd ?? input.value.length
  const newValue = input.value.substring(0, start) + text + input.value.substring(end)
  emit('update:search', newValue)
  await nextTick()
  const pos = start + text.length
  input.setSelectionRange(pos, pos)
}

const searchSelectAll = () => {
  getSearchInput()?.select()
}
</script>

<style scoped>
.app-title-bar {
  position: sticky;
  top: 0;
  z-index: 100;
  display: grid;
  grid-template-columns: minmax(0, 260px) 1fr auto;
  grid-template-areas: 'brand actions controls';
  gap: 16px;
  align-items: center;
  min-height: 64px;
  box-sizing: border-box;
  padding: 8px 20px;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.08);
  background: rgba(var(--v-theme-surface), 0.92);
  backdrop-filter: blur(20px);
  -webkit-app-region: drag;
}

.app-title-bar__brand {
  grid-area: brand;
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.app-title-bar__logo {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 12px;
  background: rgba(var(--v-theme-primary), 0.12);
  color: rgb(var(--v-theme-primary));
  flex: 0 0 auto;
}

.app-title-bar__copy {
  min-width: 0;
}

.app-title-bar__copy > * {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-title-bar__subtitle {
  font-size: 0.75rem;
  color: rgba(var(--v-theme-on-surface), 0.62);
}

.app-title-bar__actions {
  grid-area: actions;
  justify-content: flex-end;
  gap: 8px;
  min-width: 0;
}

.app-title-bar__search {
  flex-shrink: 0;
}

.app-title-bar__search--open {
  flex: 1 1 0;
  min-width: 0;
  max-width: 480px;
}

.app-title-bar__search-field {
  font-size: 0.875rem;
}

.app-title-bar__icon {
  width: 18px;
  height: 18px;
  fill: currentColor;
  display: block;
}

.app-title-bar__action-btn {
  min-width: 40px;
}

.app-title-bar__window-controls {
  grid-area: controls;
  justify-content: flex-end;
  gap: 4px;
}

.app-title-bar :deep(.v-btn),
.app-title-bar :deep(.v-field),
.app-title-bar :deep(input) {
  -webkit-app-region: no-drag;
}

.app-title-bar__actions :deep(> *) {
  flex-shrink: 0;
}

.app-title-bar__close:hover {
  background: rgba(var(--v-theme-error), 0.16);
  color: rgb(var(--v-theme-error));
}

@media (max-width: 600px) {
  .app-title-bar {
    gap: 12px;
    padding: 8px 12px;
  }

  .app-title-bar__brand {
    gap: 10px;
  }

  .app-title-bar__logo {
    width: 32px;
    height: 32px;
    border-radius: 10px;
  }

  .app-title-bar__window-controls {
    gap: 2px;
  }
}
</style>
