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
        <div class="text-h6">{{ title }}</div>
        <div v-if="subtitle" class="text-caption text-medium-emphasis">{{ subtitle }}</div>
      </div>
    </div>

    <div v-if="$slots.center" class="app-title-bar__center app-title-bar__no-drag">
      <slot name="center" />
    </div>

    <div v-if="$slots.actions" class="app-title-bar__actions app-title-bar__no-drag">
      <slot name="actions" />
    </div>

    <div v-if="isSupported" class="app-title-bar__window-controls app-title-bar__no-drag">
      <v-btn icon variant="text" size="small" :aria-label="t('titleBar.minimize')" @click="minimize">
        <v-icon icon="mdi-window-minimize" />
      </v-btn>
      <v-btn
        icon
        variant="text"
        size="small"
        :aria-label="isMaximized ? t('titleBar.restore') : t('titleBar.maximize')"
        @click="toggleMaximize"
      >
        <v-icon :icon="isMaximized ? 'mdi-window-restore' : 'mdi-window-maximize'" />
      </v-btn>
      <v-btn icon variant="text" size="small" :aria-label="t('titleBar.close')" class="app-title-bar__close" @click="close">
        <v-icon icon="mdi-close" />
      </v-btn>
    </div>
  </header>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useWindowControls } from '@/composables/useWindowControls'

const { t } = useI18n()

withDefaults(
  defineProps<{
    subtitle?: string
    title?: string
  }>(),
  {
    subtitle: undefined,
    title: 'MailYou',
  },
)

const { close, isMaximized, isSupported, minimize, toggleMaximize } = useWindowControls()
</script>

<style scoped>
.app-title-bar {
  position: sticky;
  top: 0;
  z-index: 100;
  display: grid;
  grid-template-columns: minmax(0, 260px) minmax(220px, 1fr) auto auto;
  grid-template-areas: 'brand center actions controls';
  gap: 16px;
  align-items: center;
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

.app-title-bar__center {
  grid-area: center;
  min-width: 0;
  max-width: 720px;
}

.app-title-bar__actions {
  grid-area: actions;
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 12px;
  min-width: 0;
  flex-wrap: wrap;
}

.app-title-bar__window-controls {
  grid-area: controls;
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
}

.app-title-bar__no-drag,
.app-title-bar__no-drag :deep(*) {
  -webkit-app-region: no-drag;
}

.app-title-bar__actions :deep(> *) {
  flex-shrink: 0;
}

.app-title-bar__window-controls :deep(.v-btn) {
  border-radius: 10px;
}

.app-title-bar__close:hover {
  background: rgba(var(--v-theme-error), 0.16);
  color: rgb(var(--v-theme-error));
}

@media (max-width: 1280px) {
  .app-title-bar {
    grid-template-columns: minmax(0, 1fr) auto auto;
    grid-template-areas:
      'brand actions controls'
      'center center center';
  }

  .app-title-bar__center {
    max-width: none;
  }

  .app-title-bar__actions {
    justify-content: flex-start;
  }
}

@media (max-width: 840px) {
  .app-title-bar {
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-areas:
      'brand controls'
      'actions actions'
      'center center';
    padding: 12px 16px;
  }

  .app-title-bar__actions {
    justify-content: flex-start;
  }
}

@media (max-width: 600px) {
  .app-title-bar {
    gap: 12px;
    padding: 12px;
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
