<template>
  <div class="settings-page">
    <AppTitleBar title="MailStack" subtitle="Appearance and shell preferences">
      <template #actions>
        <v-btn prepend-icon="mdi-arrow-left" @click="router.push('/')">Back to mail</v-btn>
      </template>
    </AppTitleBar>

    <v-container max-width="880" class="settings-page__content">
      <v-card class="settings-card">
        <div class="text-overline mb-2">Settings</div>
        <div class="text-h4 mb-2">Appearance and shell preferences</div>
        <div class="text-body-1 text-medium-emphasis mb-6">
          Persist lightweight UI preferences now; move mailbox and credential persistence into Rust storage later.
        </div>

        <v-row class="settings-row" dense>
          <v-col cols="12" md="6">
            <v-card class="settings-panel pa-4" color="surface">
              <div class="text-subtitle-1 mb-3">Theme mode</div>
              <v-btn-toggle class="mode-toggle" :model-value="uiStore.appearance" mandatory divided @update:model-value="setAppearance">
                <v-btn value="light" block>Light</v-btn>
                <v-btn value="dark" block>Dark</v-btn>
              </v-btn-toggle>
            </v-card>
          </v-col>

          <v-col cols="12" md="6">
            <v-card class="settings-panel pa-4" color="surface">
              <div class="text-subtitle-1 mb-3">Seed color</div>
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

          <v-col cols="12">
            <v-card class="settings-panel pa-4" color="surface">
              <div class="text-subtitle-1 mb-3">Density</div>
              <v-select
                :items="densityOptions"
                :model-value="uiStore.density"
                item-title="label"
                item-value="value"
                label="Density"
                @update:model-value="setDensity"
              />
            </v-card>
          </v-col>
        </v-row>
      </v-card>

      <v-card class="settings-card">
        <div class="text-overline mb-2">Mail</div>
        <div class="text-h4 mb-2">Sync settings</div>
        <div class="text-body-1 text-medium-emphasis mb-6">
          Configure how often MailStack syncs and how many messages to fetch per sync.
        </div>

        <v-row class="settings-row" dense>
          <v-col cols="12" md="6">
            <v-card class="settings-panel pa-4" color="surface">
              <div class="text-subtitle-1 mb-3">Sync frequency</div>
              <v-select
                :items="syncIntervalOptions"
                :model-value="uiStore.syncIntervalMinutes"
                item-title="label"
                item-value="value"
                label="Sync every"
                @update:model-value="setSyncInterval"
              />
            </v-card>
          </v-col>

          <v-col cols="12" md="6">
            <v-card class="settings-panel pa-4" color="surface">
              <div class="text-subtitle-1 mb-3">Fetch limit</div>
              <v-select
                :items="fetchLimitOptions"
                :model-value="uiStore.fetchLimit"
                item-title="label"
                item-value="value"
                label="Messages per sync"
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
import AppTitleBar from '@/components/AppTitleBar.vue'
import { useRouter } from 'vue-router'
import { useUiStore, type AppearanceMode, type DensityMode } from '@/stores/ui'

const router = useRouter()
const uiStore = useUiStore()

const densityOptions = [
  { label: 'Comfortable', value: 'comfortable' },
  { label: 'Compact', value: 'compact' },
] satisfies Array<{ label: string; value: DensityMode }>

const seedOptions = [
  { label: 'Violet', value: '#6750A4' },
  { label: 'Blue', value: '#5B8DEF' },
  { label: 'Green', value: '#0F9D58' },
  { label: 'Rose', value: '#C75B7A' },
] as const

const syncIntervalOptions = [
  { label: '1 minute', value: 1 },
  { label: '3 minutes', value: 3 },
  { label: '5 minutes', value: 5 },
  { label: '10 minutes', value: 10 },
  { label: '15 minutes', value: 15 },
  { label: '30 minutes', value: 30 },
] satisfies Array<{ label: string; value: number }>

const fetchLimitOptions = [
  { label: '25 messages', value: 25 },
  { label: '50 messages', value: 50 },
  { label: '100 messages', value: 100 },
  { label: '200 messages', value: 200 },
] satisfies Array<{ label: string; value: number }>

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
