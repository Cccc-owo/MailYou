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
        <div class="text-h5 mb-2">
          {{ securityStore.hasMissingStorageKey ? t('security.missingStorageKeyTitle') : t('security.keyringUnavailableTitle') }}
        </div>
        <div class="text-body-2 text-medium-emphasis mb-4">
          {{ securityStore.hasMissingStorageKey ? t('security.missingStorageKeyDescription') : t('security.keyringUnavailableDescription') }}
        </div>
        <v-alert type="warning" variant="tonal" class="mb-4">
          {{
            securityStore.hasMissingStorageKey
              ? t('security.missingStorageKeyDetail')
              : securityStore.status?.keyringError || t('security.keyringUnavailableGeneric')
          }}
        </v-alert>
        <v-alert
          v-if="canRestoreFromRecovery"
          type="info"
          variant="tonal"
          class="mb-4"
        >
          {{ t('security.recoveryRestoreAvailable', { count: recoverySnapshotCount }) }}
        </v-alert>
        <v-alert
          v-if="recoveryRestoreError"
          type="error"
          variant="tonal"
          class="mb-4"
        >
          {{ recoveryRestoreError }}
        </v-alert>
        <div class="d-flex ga-3 flex-wrap">
          <v-btn
            v-if="canRestoreFromRecovery"
            color="primary"
            :loading="restoringRecovery"
            @click="restoreLatestRecoveryExport"
          >
            {{ t('security.restoreLatestRecoveryAction') }}
          </v-btn>
          <v-btn
            v-if="securityStore.hasMissingStorageKey"
            color="error"
            variant="tonal"
            :loading="resettingLocalStorage"
            :disabled="restoringRecovery"
            @click="resetLocalEncryptedStorage"
          >
            {{ t('security.resetLocalStorageAction') }}
          </v-btn>
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
      <v-card class="close-prompt">
        <v-card-title class="close-prompt__title">{{ t('closePrompt.title') }}</v-card-title>
        <v-card-text class="close-prompt__body">
          <div class="close-prompt__description text-body-1">{{ t('closePrompt.description') }}</div>
          <v-checkbox
            v-model="rememberCloseBehavior"
            class="close-prompt__checkbox"
            hide-details
            :label="t('closePrompt.always')"
          />
        </v-card-text>
        <v-card-actions class="close-prompt__actions">
          <v-spacer />
          <v-btn variant="text" :disabled="rememberCloseBehavior" @click="cancelCloseRequest">
            {{ t('closePrompt.cancel') }}
          </v-btn>
          <v-btn variant="text" @click="resolveCloseRequest('background')">
            {{ t('closePrompt.minimize') }}
          </v-btn>
          <v-btn class="close-prompt__close-btn" variant="text" @click="resolveCloseRequest('quit')">
            {{ t('closePrompt.quit') }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-app>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
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
const resettingLocalStorage = ref(false)
const restoringRecovery = ref(false)
const recoverySnapshotCount = ref(0)
const recoveryRestoreError = ref<string | null>(null)
let closeRequestedUnsubscribe: (() => void) | undefined

const canRestoreFromRecovery = computed(() =>
  securityStore.hasMissingStorageKey && recoverySnapshotCount.value > 0,
)

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

const resetLocalEncryptedStorage = async () => {
  resettingLocalStorage.value = true
  try {
    await window.mailyou?.resetLocalEncryptedStorage()
    closePromptOpen.value = false
    rememberCloseBehavior.value = false
    window.location.reload()
  } finally {
    resettingLocalStorage.value = false
  }
}

const loadRecoveryExportAvailability = async () => {
  recoveryRestoreError.value = null

  if (!securityStore.hasMissingStorageKey) {
    recoverySnapshotCount.value = 0
    return
  }

  try {
    const status = await window.mailyou?.getRecoveryExportStatus()
    recoverySnapshotCount.value = status?.snapshotCount ?? 0
  } catch {
    recoverySnapshotCount.value = 0
  }
}

const restoreLatestRecoveryExport = async () => {
  restoringRecovery.value = true
  recoveryRestoreError.value = null
  try {
    await window.mailyou?.restoreLatestRecoveryExport()
    closePromptOpen.value = false
    rememberCloseBehavior.value = false
    window.location.reload()
  } catch (err) {
    recoveryRestoreError.value = err instanceof Error
      ? err.message
      : t('security.restoreLatestRecoveryFailed')
  } finally {
    restoringRecovery.value = false
  }
}

const cancelCloseRequest = async () => {
  await window.windowControls?.cancelCloseRequest()
  closePromptOpen.value = false
  rememberCloseBehavior.value = false
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
  await loadRecoveryExportAvailability()
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

watch(
  () => securityStore.hasMissingStorageKey,
  () => {
    void loadRecoveryExportAvailability()
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

.close-prompt {
  border-radius: 28px;
  padding-top: 8px;
}

.close-prompt__title {
  padding: 16px 24px 0;
  font-size: 1.75rem;
  line-height: 1.25;
  font-weight: 500;
  letter-spacing: 0;
}

.close-prompt__body {
  padding: 20px 24px 8px;
}

.close-prompt__description {
  line-height: 1.55;
  color: rgba(var(--v-theme-on-surface), 0.86);
}

.close-prompt__checkbox {
  margin-top: 16px;
}

.close-prompt__actions {
  padding: 8px 24px 20px;
  gap: 8px;
}

.close-prompt__close-btn:hover {
  background: rgba(var(--v-theme-error), 0.12);
  color: rgb(var(--v-theme-error));
}

.ui-empty-state {
  display: grid;
  place-items: center;
  text-align: center;
  min-height: 200px;
  padding: 24px;
}

.ui-empty-state__icon {
  margin-bottom: 12px;
}

.ui-empty-state__description {
  color: rgba(var(--v-theme-on-surface), 0.65);
}

.ui-empty-state__actions {
  margin-top: 16px;
}

.ui-section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: rgb(var(--v-theme-primary));
}

.ui-section-header__title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ui-surface-panel {
  border-radius: 16px;
  background: rgb(var(--v-theme-surface));
  box-shadow:
    0 1px 3px rgba(0, 0, 0, 0.06),
    0 1px 2px rgba(0, 0, 0, 0.04);
}

.ui-subtle-panel {
  border: 1px solid rgba(var(--v-theme-on-surface), 0.08);
  border-radius: 16px;
  background: rgba(var(--v-theme-on-surface), 0.02);
}

.ui-pane-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  flex-shrink: 0;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.06);
}

.ui-row {
  display: flex;
  align-items: center;
  gap: 16px;
  min-height: 56px;
  padding: 12px 16px;
}

.ui-inline-actions {
  display: flex;
  align-items: center;
}

.ui-page-content {
  max-width: 720px;
  margin: 0 auto;
  padding: 24px 24px 48px;
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

@media (max-width: 600px) {
  .ui-section-header .v-btn {
    width: 100%;
    justify-content: center;
  }
}
</style>
