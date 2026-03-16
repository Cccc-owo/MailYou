<template>
  <v-app>
    <div v-if="!securityStore.isReady" class="app-shell app-shell--centered">
      <v-progress-circular indeterminate color="primary" />
    </div>

    <div v-else-if="securityStore.error && !securityStore.status" class="app-shell app-shell--centered">
      <v-card class="app-shell__card pa-6">
        <div class="text-h6 mb-2">{{ t('security.startupFailed') }}</div>
        <div class="text-body-2 text-medium-emphasis mb-4">{{ securityStore.error }}</div>
        <v-btn color="primary" :loading="securityStore.isBusy" @click="retryInitialize">
          {{ t('common.retry') }}
        </v-btn>
      </v-card>
    </div>

    <div v-else-if="securityStore.hasKeyringIssue" class="app-shell app-shell--centered">
      <v-card class="app-shell__card pa-6">
        <div class="text-overline mb-2">{{ t('security.title') }}</div>
        <div class="text-h5 mb-2">{{ t('security.keyringUnavailableTitle') }}</div>
        <div class="text-body-2 text-medium-emphasis mb-4">
          {{ t('security.keyringUnavailableDescription') }}
        </div>
        <v-alert type="warning" variant="tonal" class="mb-4">
          {{ securityStore.status?.keyringError || t('security.keyringUnavailableGeneric') }}
        </v-alert>
        <div class="d-flex ga-3 flex-wrap">
          <v-btn color="primary" @click="goToSettings">
            {{ t('security.openSecuritySettings') }}
          </v-btn>
          <v-btn variant="text" :loading="securityStore.isBusy" @click="retryInitialize">
            {{ t('common.retry') }}
          </v-btn>
        </div>
      </v-card>
    </div>

    <div v-else-if="securityStore.requiresUnlock" class="app-shell app-shell--centered">
      <v-card class="app-shell__card pa-6">
        <div class="text-overline mb-2">{{ t('security.title') }}</div>
        <div class="text-h5 mb-2">{{ t('security.unlockTitle') }}</div>
        <div class="text-body-2 text-medium-emphasis mb-5">{{ t('security.unlockDescription') }}</div>
        <v-form @submit.prevent="submitUnlock">
          <v-text-field
            v-model="unlockPassword"
            :label="t('security.masterPassword')"
            type="password"
            autocomplete="current-password"
            autofocus
          />
          <v-alert v-if="securityStore.error" type="error" variant="tonal" class="mb-4">
            {{ securityStore.error }}
          </v-alert>
          <v-btn block color="primary" type="submit" :loading="securityStore.isBusy">
            {{ t('security.unlockAction') }}
          </v-btn>
        </v-form>
      </v-card>
    </div>

    <router-view v-else />
  </v-app>
</template>

<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useThemeController } from '@/composables/useThemeController'
import { useSecurityStore } from '@/stores/security'
import { useUiStore } from '@/stores/ui'

useThemeController()

const { t } = useI18n()
const router = useRouter()
const securityStore = useSecurityStore()
const uiStore = useUiStore()
const unlockPassword = ref('')

const retryInitialize = async () => {
  await securityStore.initialize()
}

const goToSettings = async () => {
  await router.push('/settings')
}

const submitUnlock = async () => {
  if (!unlockPassword.value) return
  await securityStore.unlock(unlockPassword.value)
  unlockPassword.value = ''
}

onMounted(async () => {
  await securityStore.initialize()
  await window.windowControls?.setBackgroundSyncInterval(uiStore.syncIntervalMinutes)
})

watch(
  () => uiStore.syncIntervalMinutes,
  (minutes) => {
    void window.windowControls?.setBackgroundSyncInterval(minutes)
  },
)
</script>

<style>
.app-shell {
  min-height: 100vh;
  background:
    radial-gradient(circle at top, rgba(var(--v-theme-primary), 0.12), transparent 38%),
    linear-gradient(180deg, rgb(var(--v-theme-background)), rgb(var(--v-theme-surface)));
}

.app-shell--centered {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.app-shell__card {
  width: min(420px, 100%);
  border-radius: 24px;
}

/* Custom scrollbar — thin, theme-aware, applied globally */
*,
*::before,
*::after {
  scrollbar-width: thin;
  scrollbar-color: rgba(var(--v-theme-on-surface), 0.2) transparent;
}

::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(var(--v-theme-on-surface), 0.15);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(var(--v-theme-on-surface), 0.3);
}

::-webkit-scrollbar-corner {
  background: transparent;
}

.v-tooltip > .v-overlay__content {
  background: #1a1a1a !important;
  color: #f0f0f0 !important;
  font-size: 0.75rem;
  padding: 4px 10px;
  border-radius: 6px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
  pointer-events: none;
}
</style>
