<template>
  <div class="settings-page">
    <AppTitleBar title="MailYou" :subtitle="t('settings.subtitle')">
      <template #actions>
        <v-btn prepend-icon="mdi-arrow-left" @click="router.push('/')">{{ t('common.backToMail') }}</v-btn>
      </template>
    </AppTitleBar>

    <v-container max-width="880" class="settings-page__content">
      <v-card class="settings-card">
        <div class="text-overline mb-2">{{ t('settings.title') }}</div>
        <div class="text-h4 mb-2">{{ t('settings.subtitle') }}</div>
        <div class="text-body-1 text-medium-emphasis mb-6">
          {{ t('settings.description') }}
        </div>

        <v-row class="settings-row" dense>
          <v-col cols="12" md="6">
            <v-card class="settings-panel pa-4" color="surface">
              <div class="text-subtitle-1 mb-3">{{ t('settings.themeMode') }}</div>
              <v-btn-toggle class="mode-toggle" :model-value="uiStore.appearance" mandatory divided @update:model-value="setAppearance">
                <v-btn value="light" block>{{ t('settings.light') }}</v-btn>
                <v-btn value="dark" block>{{ t('settings.dark') }}</v-btn>
              </v-btn-toggle>
            </v-card>
          </v-col>

          <v-col cols="12" md="6">
            <v-card class="settings-panel pa-4" color="surface">
              <div class="text-subtitle-1 mb-3">{{ t('settings.language') }}</div>
              <v-btn-toggle class="mode-toggle" :model-value="uiStore.locale" mandatory divided @update:model-value="setLocale">
                <v-btn value="en" block>English</v-btn>
                <v-btn value="zh" block>简体中文</v-btn>
              </v-btn-toggle>
            </v-card>
          </v-col>

          <v-col cols="12" md="6">
            <v-card class="settings-panel pa-4" color="surface">
              <div class="text-subtitle-1 mb-3">{{ t('settings.seedColor') }}</div>
              <v-item-group :model-value="uiStore.themeSeed" mandatory @update:model-value="setThemeSeed">
                <v-row dense>
                  <v-col v-for="option in seedOptions" :key="option.value" cols="12" sm="6">
                    <v-item v-slot="{ isSelected, toggle }" :value="option.value">
                      <v-btn
                        :color="isSelected ? 'primary' : undefined"
                        :variant="isSelected ? 'flat' : 'tonal'"
                        block
                        class="seed-option"
                        height="56"
                        @click="toggle"
                      >
                        <span class="seed-option__content">
                          <span class="seed-swatch" :style="{ backgroundColor: option.value }" />
                          <span>{{ option.label }}</span>
                        </span>
                      </v-btn>
                    </v-item>
                  </v-col>
                </v-row>
              </v-item-group>
            </v-card>
          </v-col>

          <v-col cols="12" md="6">
            <v-card class="settings-panel pa-4" color="surface">
              <div class="text-subtitle-1 mb-3">{{ t('settings.density') }}</div>
              <v-select
                :items="densityOptions"
                :model-value="uiStore.density"
                item-title="label"
                item-value="value"
                :label="t('settings.density')"
                @update:model-value="setDensity"
              />
            </v-card>
          </v-col>
        </v-row>
      </v-card>

      <v-card class="settings-card">
        <div class="text-overline mb-2">{{ t('settings.mail') }}</div>
        <div class="text-h4 mb-2">{{ t('settings.syncSettings') }}</div>
        <div class="text-body-1 text-medium-emphasis mb-6">
          {{ t('settings.syncDescription') }}
        </div>

        <v-row class="settings-row" dense>
          <v-col cols="12" md="6">
            <v-card class="settings-panel pa-4" color="surface">
              <div class="text-subtitle-1 mb-3">{{ t('settings.syncFrequency') }}</div>
              <v-select
                :items="syncIntervalOptions"
                :model-value="uiStore.syncIntervalMinutes"
                item-title="label"
                item-value="value"
                :label="t('settings.syncEvery')"
                @update:model-value="setSyncInterval"
              />
            </v-card>
          </v-col>

          <v-col cols="12" md="6">
            <v-card class="settings-panel pa-4" color="surface">
              <div class="text-subtitle-1 mb-3">{{ t('settings.fetchLimit') }}</div>
              <v-select
                :items="fetchLimitOptions"
                :model-value="uiStore.fetchLimit"
                item-title="label"
                item-value="value"
                :label="t('settings.messagesPerSync')"
                @update:model-value="setFetchLimit"
              />
            </v-card>
          </v-col>
        </v-row>
      </v-card>
    </v-container>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import AppTitleBar from '@/components/AppTitleBar.vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useUiStore, type AppearanceMode, type DensityMode, type LocaleMode } from '@/stores/ui'

const { t } = useI18n()
const router = useRouter()
const uiStore = useUiStore()

const densityOptions = computed(() => [
  { label: t('settings.comfortable'), value: 'comfortable' },
  { label: t('settings.compact'), value: 'compact' },
] satisfies Array<{ label: string; value: DensityMode }>)

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

const setThemeSeed = (value: string | null) => {
  if (!value) {
    return
  }

  uiStore.setThemeSeed(value)
}

const setAppearance = (value: AppearanceMode | null) => {
  if (!value) {
    return
  }

  uiStore.setAppearance(value)
}

const setDensity = (value: DensityMode | null) => {
  if (!value) {
    return
  }

  uiStore.setDensity(value)
}

const setLocale = (value: LocaleMode | null) => {
  if (!value) {
    return
  }

  uiStore.setLocale(value)
}

const setSyncInterval = (value: number | null) => {
  if (!value) {
    return
  }

  uiStore.setSyncIntervalMinutes(value)
}

const setFetchLimit = (value: number | null) => {
  if (!value) {
    return
  }

  uiStore.setFetchLimit(value)
}
</script>

<style scoped>
.settings-page {
  min-height: 100vh;
  background: rgb(var(--v-theme-background));
}

.settings-page__content {
  display: grid;
  gap: 24px;
  padding-block: 40px;
}

.settings-card {
  padding: 32px;
}

.settings-row {
  margin: 0;
}

.settings-panel {
  height: 100%;
  color: rgb(var(--v-theme-on-surface));
}

.mode-toggle {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  width: 100%;
}

.mode-toggle :deep(.v-btn) {
  min-width: 0;
}

.seed-option {
  min-width: 0;
}

.seed-option__content {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  width: 100%;
  text-align: center;
  white-space: normal;
}

.seed-swatch {
  width: 16px;
  height: 16px;
  border-radius: 999px;
  display: inline-block;
  flex: 0 0 auto;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.14);
}

@media (max-width: 840px) {
  .settings-page__content {
    gap: 20px;
    padding-block: 28px;
  }

  .settings-card {
    padding: 24px;
  }
}

@media (max-width: 600px) {
  .settings-page__content {
    padding-block: 20px;
  }

  .settings-card {
    padding: 16px;
  }

  .mode-toggle {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
