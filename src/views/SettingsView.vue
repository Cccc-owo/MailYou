<template>
  <div class="settings-page">
    <AppTitleBar title="MailYou">
      <template #actions>
        <v-tooltip :text="t('common.backToMail')" location="bottom">
          <template #activator="{ props: tip }">
            <v-btn v-bind="tip" icon="mdi-arrow-left" @click="router.push('/')" />
          </template>
        </v-tooltip>
      </template>
    </AppTitleBar>

    <div class="settings-page__content">
      <!-- Appearance -->
      <div class="settings-page__section">
        <div class="settings-page__section-header">
          <v-icon icon="mdi-palette-outline" size="18" />
          <span>{{ t('settings.title') }}</span>
        </div>

        <div class="settings-page__group">
          <div class="settings-page__item">
            <v-icon icon="mdi-brightness-6" class="settings-page__item-icon" />
            <div class="settings-page__item-body">
              <div class="settings-page__item-label">{{ t('settings.themeMode') }}</div>
            </div>
            <v-btn-toggle :model-value="uiStore.appearance" mandatory variant="outlined" divided @update:model-value="setAppearance">
              <v-btn value="light">{{ t('settings.light') }}</v-btn>
              <v-btn value="dark">{{ t('settings.dark') }}</v-btn>
            </v-btn-toggle>
          </div>

          <v-divider class="settings-page__divider" />

          <div class="settings-page__item">
            <v-icon icon="mdi-translate" class="settings-page__item-icon" />
            <div class="settings-page__item-body">
              <div class="settings-page__item-label">{{ t('settings.language') }}</div>
            </div>
            <v-btn-toggle :model-value="uiStore.locale" mandatory variant="outlined" divided @update:model-value="setLocale">
              <v-btn value="en">English</v-btn>
              <v-btn value="zh">简体中文</v-btn>
            </v-btn-toggle>
          </div>

          <v-divider class="settings-page__divider" />

          <div class="settings-page__item settings-page__item--vertical">
            <div class="settings-page__item-row">
              <v-icon icon="mdi-palette-swatch-outline" class="settings-page__item-icon" />
              <div class="settings-page__item-body">
                <div class="settings-page__item-label">{{ t('settings.seedColor') }}</div>
              </div>
            </div>
            <div class="settings-page__color-row">
              <button
                v-for="option in seedOptions"
                :key="option.value"
                class="settings-page__color-option"
                :class="{ 'settings-page__color-option--active': uiStore.themeSeed === option.value }"
                :style="{ '--chip-color': option.value }"
                @click="setThemeSeed(option.value)"
              >
                <span class="settings-page__color-swatch" />
                <span class="settings-page__color-name">{{ option.label }}</span>
                <v-icon v-if="uiStore.themeSeed === option.value" icon="mdi-check" size="16" />
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Sync -->
      <div class="settings-page__section">
        <div class="settings-page__section-header">
          <v-icon icon="mdi-sync" size="18" />
          <span>{{ t('settings.syncSettings') }}</span>
        </div>

        <div class="settings-page__group">
          <div class="settings-page__item">
            <v-icon icon="mdi-timer-outline" class="settings-page__item-icon" />
            <div class="settings-page__item-body">
              <div class="settings-page__item-label">{{ t('settings.syncFrequency') }}</div>
            </div>
            <v-select
              :items="syncIntervalOptions"
              :model-value="uiStore.syncIntervalMinutes"
              item-title="label"
              item-value="value"
              density="compact"
              hide-details
              variant="outlined"
              class="settings-page__select"
              @update:model-value="setSyncInterval"
            />
          </div>

          <v-divider class="settings-page__divider" />

          <div class="settings-page__item">
            <v-icon icon="mdi-email-multiple-outline" class="settings-page__item-icon" />
            <div class="settings-page__item-body">
              <div class="settings-page__item-label">{{ t('settings.fetchLimit') }}</div>
            </div>
            <v-select
              :items="fetchLimitOptions"
              :model-value="uiStore.fetchLimit"
              item-title="label"
              item-value="value"
              density="compact"
              hide-details
              variant="outlined"
              class="settings-page__select"
              @update:model-value="setFetchLimit"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import AppTitleBar from '@/components/AppTitleBar.vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useUiStore, type AppearanceMode, type LocaleMode } from '@/stores/ui'

const { t } = useI18n()
const router = useRouter()
const uiStore = useUiStore()

const seedOptions = computed(() => [
  { label: t('settings.violet'), value: '#6750A4' },
  { label: t('settings.blue'), value: '#5B8DEF' },
  { label: t('settings.green'), value: '#0F9D58' },
  { label: t('settings.rose'), value: '#C75B7A' },
] as const)

const syncIntervalOptions = computed(() => [
  { label: t('settings.syncInterval1'), value: 1 },
  { label: t('settings.syncInterval3'), value: 3 },
  { label: t('settings.syncInterval5'), value: 5 },
  { label: t('settings.syncInterval10'), value: 10 },
  { label: t('settings.syncInterval15'), value: 15 },
  { label: t('settings.syncInterval30'), value: 30 },
] satisfies Array<{ label: string; value: number }>)

const fetchLimitOptions = computed(() => [
  { label: t('settings.fetchLimit25'), value: 25 },
  { label: t('settings.fetchLimit50'), value: 50 },
  { label: t('settings.fetchLimit100'), value: 100 },
  { label: t('settings.fetchLimit200'), value: 200 },
] satisfies Array<{ label: string; value: number }>)

const setThemeSeed = (value: string) => uiStore.setThemeSeed(value)
const setAppearance = (value: AppearanceMode | null) => { if (value) uiStore.setAppearance(value) }
const setLocale = (value: LocaleMode | null) => { if (value) uiStore.setLocale(value) }
const setSyncInterval = (value: number | null) => { if (value) uiStore.setSyncIntervalMinutes(value) }
const setFetchLimit = (value: number | null) => { if (value) uiStore.setFetchLimit(value) }
</script>

<style scoped>
.settings-page {
  min-height: 100vh;
  background: rgb(var(--v-theme-background));
}

.settings-page__content {
  max-width: 720px;
  margin: 0 auto;
  padding: 24px 24px 48px;
  display: flex;
  flex-direction: column;
  gap: 32px;
}

/* ── Section header (MD3 label-small, primary) ── */

.settings-page__section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: rgb(var(--v-theme-primary));
  padding: 0 16px;
  margin-bottom: 8px;
}

/* ── Group container (surface, single-level) ── */

.settings-page__group {
  border-radius: 16px;
  background: rgb(var(--v-theme-surface));
  box-shadow:
    0 1px 3px rgba(0, 0, 0, 0.06),
    0 1px 2px rgba(0, 0, 0, 0.04);
}

/* ── List item (MD3: 56px min-height, 16px padding) ── */

.settings-page__item {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 16px;
  min-height: 56px;
}

.settings-page__item--vertical {
  flex-direction: column;
  align-items: stretch;
  gap: 12px;
}

.settings-page__item-row {
  display: flex;
  align-items: center;
  gap: 16px;
}

.settings-page__item-icon {
  flex-shrink: 0;
  opacity: 0.64;
}

.settings-page__item-body {
  flex: 1;
  min-width: 0;
}

.settings-page__item-label {
  font-size: 1rem;
  line-height: 1.5;
}

.settings-page__divider {
  margin-inline: 56px 16px;
}

/* ── Segmented buttons (MD3 outlined) ── */

.settings-page__item :deep(.v-btn-toggle) {
  flex-shrink: 0;
  height: 36px;
  border-radius: 18px;
}

.settings-page__item :deep(.v-btn-toggle .v-btn) {
  min-width: 64px;
  font-size: 0.8125rem;
  letter-spacing: 0.02em;
  text-transform: none;
}

/* ── Select (fixed width, compact) ── */

.settings-page__select {
  max-width: 180px;
  flex-shrink: 0;
}

/* ── Color palette row ── */

.settings-page__color-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  padding-left: 40px;
}

.settings-page__color-option {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 40px;
  padding: 0 16px;
  border-radius: 20px;
  border: 1.5px solid rgba(var(--v-theme-on-surface), 0.12);
  background: transparent;
  color: inherit;
  font-size: 0.875rem;
  cursor: pointer;
  transition: border-color 0.2s, background 0.2s;
}

.settings-page__color-option:hover {
  background: rgba(var(--v-theme-on-surface), 0.05);
}

.settings-page__color-option--active {
  border-color: var(--chip-color);
  background: color-mix(in srgb, var(--chip-color) 12%, transparent);
  font-weight: 500;
}

.settings-page__color-swatch {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--chip-color);
  flex-shrink: 0;
  box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.08);
}

.settings-page__color-name {
  flex: 1;
}

/* ── Responsive ── */

@media (max-width: 600px) {
  .settings-page__content {
    padding: 16px 16px 32px;
    gap: 24px;
  }

  .settings-page__item {
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    min-height: auto;
  }

  .settings-page__item--vertical {
    gap: 12px;
  }

  .settings-page__divider {
    margin-inline: 16px;
  }

  .settings-page__color-row {
    padding-left: 0;
  }
}
</style>
