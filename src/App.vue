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

    <v-dialog v-model="closePromptOpen" max-width="460" persistent>
      <v-card>
        <v-card-title>{{ t('closePrompt.title') }}</v-card-title>
        <v-card-text>
          <div class="text-body-1 mb-4">{{ t('closePrompt.description') }}</div>
          <v-checkbox
            v-model="rememberCloseBehavior"
            hide-details
            :label="t('closePrompt.alwaysBackground')"
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="resolveCloseRequest('quit')">
            {{ t('closePrompt.quit') }}
          </v-btn>
          <v-btn color="primary" variant="flat" @click="resolveCloseRequest('background')">
            {{ t('closePrompt.keepRunning') }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-app>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useThemeController } from '@/composables/useThemeController'
import { useSecurityStore } from '@/stores/security'
import { useUiStore } from '@/stores/ui'
import type { CloseRequestAction } from '@/shared/window/bridge'

useThemeController()

const { t } = useI18n()
const router = useRouter()
const securityStore = useSecurityStore()
const uiStore = useUiStore()
const unlockPassword = ref('')
const closePromptOpen = ref(false)
const rememberCloseBehavior = ref(false)
let closeRequestedUnsubscribe: (() => void) | undefined

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

const resolveCloseRequest = async (action: CloseRequestAction) => {
  await window.windowControls?.resolveCloseRequest(action, rememberCloseBehavior.value)
  if (action === 'background' && rememberCloseBehavior.value) {
    uiStore.setCloseBehavior('always_background')
  }
  closePromptOpen.value = false
  rememberCloseBehavior.value = false
}

onMounted(async () => {
  await securityStore.initialize()
  await window.windowControls?.setBackgroundSyncInterval(uiStore.syncIntervalMinutes)
  await window.windowControls?.setCloseBehaviorPreference(uiStore.closeBehavior)
  closeRequestedUnsubscribe = window.windowControls?.onCloseRequested(() => {
    rememberCloseBehavior.value = false
    closePromptOpen.value = true
  })
})

watch(
  () => uiStore.syncIntervalMinutes,
  (minutes) => {
    void window.windowControls?.setBackgroundSyncInterval(minutes)
  },
)

watch(
  () => uiStore.closeBehavior,
  (value) => {
    void window.windowControls?.setCloseBehaviorPreference(value)
  },
)

onUnmounted(() => {
  closeRequestedUnsubscribe?.()
  closeRequestedUnsubscribe = undefined
})
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

/* Global scrollbar tokens */
:root {
  --app-scrollbar-size: 8px;
  --app-scrollbar-radius: 999px;
  --app-scrollbar-thumb: rgba(var(--v-theme-on-surface), 0.18);
  --app-scrollbar-thumb-hover: rgba(var(--v-theme-on-surface), 0.32);
  --app-scrollbar-track: rgba(var(--v-theme-on-surface), 0.04);
}

/* Custom scrollbar — theme-aware, applied globally */
html,
body,
#app,
.v-application,
.v-application__wrap,
.v-overlay__content,
.v-overlay__content *,
.v-menu > .v-overlay__content,
.v-dialog > .v-overlay__content,
.v-autocomplete__content,
.v-select__content,
*,
*::before,
*::after {
  scrollbar-width: thin;
  scrollbar-color: var(--app-scrollbar-thumb) var(--app-scrollbar-track);
}

::-webkit-scrollbar,
.v-overlay__content::-webkit-scrollbar,
.v-overlay__content *::-webkit-scrollbar {
  width: var(--app-scrollbar-size);
  height: var(--app-scrollbar-size);
}

::-webkit-scrollbar-track,
.v-overlay__content::-webkit-scrollbar-track,
.v-overlay__content *::-webkit-scrollbar-track {
  background: var(--app-scrollbar-track);
}

::-webkit-scrollbar-thumb,
.v-overlay__content::-webkit-scrollbar-thumb,
.v-overlay__content *::-webkit-scrollbar-thumb {
  background: var(--app-scrollbar-thumb);
  border-radius: var(--app-scrollbar-radius);
  border: 2px solid transparent;
  background-clip: padding-box;
}

::-webkit-scrollbar-thumb:hover,
.v-overlay__content::-webkit-scrollbar-thumb:hover,
.v-overlay__content *::-webkit-scrollbar-thumb:hover {
  background: var(--app-scrollbar-thumb-hover);
  background-clip: padding-box;
}

::-webkit-scrollbar-corner,
.v-overlay__content::-webkit-scrollbar-corner,
.v-overlay__content *::-webkit-scrollbar-corner {
  background: var(--app-scrollbar-track);
}

html,
body,
#app {
  background: rgb(var(--v-theme-background));
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
