<template>
  <AppScrollablePage title="MailYou" hide-search :max-width="820" content-class="settings-page__content">
    <template #actions>
      <BackActionButton :label="t('common.backToMail')" @click="router.push('/')" />
    </template>

    <!-- Appearance -->
    <FormSectionCard :title="t('settings.title')" icon="mdi-palette-outline">
          <div class="settings-page__item ui-row">
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

          <div class="settings-page__item ui-row">
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

          <div class="settings-page__item ui-row settings-page__item--vertical">
            <div class="settings-page__item-row ui-row">
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
    </FormSectionCard>

    <!-- Sync -->
    <FormSectionCard :title="t('settings.syncSettings')" icon="mdi-sync">
          <div class="settings-page__item ui-row">
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

          <div v-if="autoLaunchSupported" class="settings-page__item ui-row">
            <v-icon icon="mdi-rocket-launch-outline" class="settings-page__item-icon" />
            <div class="settings-page__item-body">
              <div class="settings-page__item-label">{{ t('settings.launchOnStartup') }}</div>
              <div class="text-body-2 text-medium-emphasis">
                {{ t('settings.launchOnStartupDescription') }}
              </div>
              <div v-if="autoLaunchError" class="text-body-2 text-error mt-1">
                {{ autoLaunchError }}
              </div>
            </div>
            <v-switch
              :model-value="autoLaunchEnabled"
              color="primary"
              hide-details
              inset
              :loading="autoLaunchLoading"
              :disabled="autoLaunchLoading"
              @update:model-value="toggleAutoLaunch"
            />
          </div>

          <v-divider v-if="autoLaunchSupported" class="settings-page__divider" />

          <div v-if="accountsStore.currentAccountId" class="settings-page__item ui-row">
            <v-icon icon="mdi-database-outline" class="settings-page__item-icon" />
            <div class="settings-page__item-body">
              <div class="settings-page__item-label">{{ t('settings.accountQuota') }}</div>
              <div class="text-body-2 text-medium-emphasis">
                {{ quotaSummary }}
              </div>
            </div>
            <v-progress-circular
              v-if="quotaLoading"
              indeterminate
              size="18"
              width="2"
              color="primary"
            />
            <div v-else-if="quota?.usagePercent != null" class="settings-page__metric">
              {{ quota.usagePercent }}%
            </div>
          </div>

          <v-divider v-if="accountsStore.currentAccountId" class="settings-page__divider" />

          <div class="settings-page__item ui-row">
            <v-icon icon="mdi-window-close" class="settings-page__item-icon" />
            <div class="settings-page__item-body">
              <div class="settings-page__item-label">{{ t('settings.closeBehavior') }}</div>
            </div>
            <v-select
              :items="closeBehaviorOptions"
              :model-value="uiStore.closeBehavior"
              item-title="label"
              item-value="value"
              density="compact"
              hide-details
              variant="outlined"
              class="settings-page__select"
              @update:model-value="setCloseBehavior"
            />
          </div>
    </FormSectionCard>

    <!-- Privacy -->
    <FormSectionCard :title="t('settings.privacy')" icon="mdi-shield-outline">
          <div class="settings-page__item ui-row">
            <v-icon icon="mdi-image-outline" class="settings-page__item-icon" />
            <div class="settings-page__item-body">
              <div class="settings-page__item-label">{{ t('settings.imageLoadPolicy') }}</div>
            </div>
            <v-select
              :items="imageLoadOptions"
              :model-value="uiStore.imageLoadPolicy"
              item-title="label"
              item-value="value"
              density="compact"
              hide-details
              variant="outlined"
              class="settings-page__select"
              @update:model-value="setImageLoadPolicy"
            />
          </div>

          <v-divider class="settings-page__divider" />

          <div class="settings-page__item ui-row settings-page__item--vertical">
            <div class="settings-page__item-row ui-row">
              <v-icon icon="mdi-lock-outline" class="settings-page__item-icon" />
              <div class="settings-page__item-body">
                <div class="settings-page__item-label">{{ t('security.title') }}</div>
                <div class="text-body-2 text-medium-emphasis">
                  {{ securitySummary }}
                </div>
              </div>
            </div>

            <div class="settings-page__security-form">
              <v-alert
                v-if="!securityStore.status?.hasMasterPassword && securityStore.status && !securityStore.status.keyringAvailable"
                type="warning"
                variant="tonal"
              >
                {{ securityStore.status.keyringError || t('security.keyringUnavailableGeneric') }}
              </v-alert>
              <v-text-field
                v-if="securityStore.status?.hasMasterPassword"
                v-model="currentPassword"
                :label="t('security.currentPassword')"
                type="password"
                autocomplete="current-password"
              />
              <v-text-field
                v-model="newPassword"
                :label="securityStore.status?.hasMasterPassword ? t('security.newPassword') : t('security.masterPassword')"
                type="password"
                autocomplete="new-password"
              />
              <v-text-field
                v-model="confirmPassword"
                :label="t('security.confirmPassword')"
                type="password"
                autocomplete="new-password"
              />

              <v-alert v-if="securityError" type="error" variant="tonal">
                {{ securityError }}
              </v-alert>
              <v-alert v-else type="info" variant="tonal">
                {{ t('security.optionalHint') }}
              </v-alert>

              <div class="d-flex ga-3 flex-wrap">
                <v-btn
                  color="primary"
                  :loading="securityStore.isBusy"
                  :disabled="!canSubmitPassword"
                  @click="saveMasterPassword"
                >
                  {{ securityStore.status?.hasMasterPassword ? t('security.updateAction') : t('security.enableAction') }}
                </v-btn>
                <v-btn
                  v-if="securityStore.status?.hasMasterPassword && securityStore.status?.isUnlocked"
                  variant="tonal"
                  :loading="securityStore.isBusy"
                  @click="lockCurrentSession"
                >
                  {{ t('security.lockNowAction') }}
                </v-btn>
                <v-btn
                  v-if="securityStore.status?.hasMasterPassword"
                  variant="text"
                  color="error"
                  :loading="securityStore.isBusy"
                  :disabled="!currentPassword"
                  @click="removeMasterPassword"
                >
                  {{ t('security.disableAction') }}
                </v-btn>
              </div>
            </div>
          </div>
    </FormSectionCard>
  </AppScrollablePage>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import AppScrollablePage from '@/components/ui/AppScrollablePage.vue'
import BackActionButton from '@/components/ui/BackActionButton.vue'
import FormSectionCard from '@/components/ui/FormSectionCard.vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { mailRepository } from '@/services/mail'
import { useAccountsStore } from '@/stores/accounts'
import { useUiStore, type AppearanceMode, type LocaleMode, type ImageLoadPolicy } from '@/stores/ui'
import { useSecurityStore } from '@/stores/security'
import type { AccountQuota } from '@/types/account'
import type { AutoLaunchSettings, CloseBehaviorPreference } from '@/shared/window/bridge'

const { t } = useI18n()
const router = useRouter()
const accountsStore = useAccountsStore()
const uiStore = useUiStore()
const securityStore = useSecurityStore()
const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const securityError = ref<string | null>(null)
const quota = ref<AccountQuota | null>(null)
const quotaLoading = ref(false)
const quotaError = ref<string | null>(null)
const autoLaunchEnabled = ref(false)
const autoLaunchSupported = ref(false)
const autoLaunchLoading = ref(false)
const autoLaunchError = ref<string | null>(null)

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

const imageLoadOptions = computed(() => [
  { label: t('settings.imageLoadNoRemote'), value: 'noRemote' },
  { label: t('settings.imageLoadNoHttp'), value: 'noHttp' },
  { label: t('settings.imageLoadAll'), value: 'all' },
] satisfies Array<{ label: string; value: string }>)

const closeBehaviorOptions = computed(() => [
  { label: t('settings.closeBehaviorAsk'), value: 'ask' },
  { label: t('settings.closeBehaviorBackground'), value: 'always_background' },
  { label: t('settings.closeBehaviorQuit'), value: 'always_quit' },
] satisfies Array<{ label: string; value: CloseBehaviorPreference }>)

const formatQuotaValue = (valueKb: number | null | undefined) => {
  if (valueKb == null) {
    return null
  }

  const units = ['KB', 'MB', 'GB', 'TB']
  let value = valueKb
  let unitIndex = 0

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024
    unitIndex += 1
  }

  const digits = value >= 100 ? 0 : value >= 10 ? 1 : 2
  return `${new Intl.NumberFormat(undefined, {
    maximumFractionDigits: digits,
    minimumFractionDigits: 0,
  }).format(value)} ${units[unitIndex]}`
}

const quotaSummary = computed(() => {
  if (quotaLoading.value) {
    return t('settings.accountQuotaLoading')
  }

  if (quotaError.value) {
    return quotaError.value
  }

  if (!accountsStore.currentAccountId) {
    return t('settings.accountQuotaUnavailable')
  }

  if (!quota.value?.storageUsedKb && quota.value?.storageUsedKb !== 0) {
    return t('settings.accountQuotaUnavailable')
  }

  const used = formatQuotaValue(quota.value.storageUsedKb)
  const limit = formatQuotaValue(quota.value.storageLimitKb)

  if (used && limit) {
    return t('settings.accountQuotaUsage', { used, limit })
  }

  if (used) {
    return t('settings.accountQuotaUsedOnly', { used })
  }

  return t('settings.accountQuotaUnavailable')
})

const setThemeSeed = (value: string) => uiStore.setThemeSeed(value)
const setAppearance = (value: AppearanceMode | null) => { if (value) uiStore.setAppearance(value) }
const setLocale = (value: LocaleMode | null) => { if (value) uiStore.setLocale(value) }
const setSyncInterval = (value: number | null) => { if (value) uiStore.setSyncIntervalMinutes(value) }
const setImageLoadPolicy = (value: ImageLoadPolicy | null) => { if (value) uiStore.setImageLoadPolicy(value) }
const setCloseBehavior = (value: CloseBehaviorPreference | null) => { if (value) uiStore.setCloseBehavior(value) }

const applyAutoLaunchSettings = (settings: AutoLaunchSettings | null | undefined) => {
  autoLaunchEnabled.value = Boolean(settings?.enabled)
  autoLaunchSupported.value = Boolean(settings?.supported)
}

const loadAutoLaunchSettings = async () => {
  autoLaunchLoading.value = true
  autoLaunchError.value = null

  try {
    applyAutoLaunchSettings(await window.windowControls?.getAutoLaunchSettings())
  } catch (err) {
    autoLaunchError.value = err instanceof Error ? err.message : t('settings.launchOnStartupFailed')
  } finally {
    autoLaunchLoading.value = false
  }
}

const toggleAutoLaunch = async (value: boolean | null) => {
  if (typeof value !== 'boolean' || autoLaunchLoading.value) {
    return
  }

  const previous = autoLaunchEnabled.value
  autoLaunchEnabled.value = value
  autoLaunchLoading.value = true
  autoLaunchError.value = null

  try {
    applyAutoLaunchSettings(await window.windowControls?.setAutoLaunchEnabled(value))
  } catch (err) {
    autoLaunchEnabled.value = previous
    autoLaunchError.value = err instanceof Error ? err.message : t('settings.launchOnStartupFailed')
  } finally {
    autoLaunchLoading.value = false
  }
}

const loadQuota = async (accountId: string | null) => {
  quota.value = null
  quotaError.value = null

  if (!accountId) {
    return
  }

  quotaLoading.value = true
  try {
    quota.value = await mailRepository.getAccountQuota(accountId)
  } catch (err) {
    quotaError.value = err instanceof Error ? err.message : t('settings.accountQuotaUnavailable')
  } finally {
    quotaLoading.value = false
  }
}

const securitySummary = computed(() =>
  !securityStore.status?.hasMasterPassword && securityStore.status && !securityStore.status.keyringAvailable
    ? t('security.keyringUnavailableSummary')
    : securityStore.status?.hasMasterPassword
    ? t('security.enabledSummary')
    : t('security.disabledSummary'),
)

const canSubmitPassword = computed(() =>
  newPassword.value.length >= 8 && newPassword.value === confirmPassword.value,
)

const resetSecurityForm = () => {
  currentPassword.value = ''
  newPassword.value = ''
  confirmPassword.value = ''
}

const saveMasterPassword = async () => {
  securityError.value = null
  if (!canSubmitPassword.value) {
    securityError.value = t('security.passwordMismatch')
    return
  }

  try {
    await securityStore.setMasterPassword(
      securityStore.status?.hasMasterPassword ? currentPassword.value : null,
      newPassword.value,
    )
    resetSecurityForm()
  } catch (err) {
    securityError.value = err instanceof Error ? err.message : t('security.updateFailed')
  }
}

const removeMasterPassword = async () => {
  securityError.value = null
  try {
    await securityStore.clearMasterPassword(currentPassword.value)
    resetSecurityForm()
  } catch (err) {
    securityError.value = err instanceof Error ? err.message : t('security.disableFailed')
  }
}

const lockCurrentSession = async () => {
  securityError.value = null
  try {
    await securityStore.lockCurrentSession()
    window.location.reload()
  } catch (err) {
    securityError.value = err instanceof Error ? err.message : t('security.lockFailed')
  }
}

watch(() => accountsStore.currentAccountId, loadQuota, { immediate: true })
void loadAutoLaunchSettings()
</script>

<style scoped>
.settings-page__content {
  display: flex;
  flex-direction: column;
  gap: 0;
  padding-top: 4px;
}

.settings-page__content > .form-section-card + .form-section-card {
  margin-top: 20px;
}

/* ── Section header (MD3 label-small, primary) ── */

/* ── List item (MD3: 56px min-height, 16px padding) ── */

.settings-page__item {
  padding-block: 14px;
}

.settings-page__item--vertical {
  flex-direction: column;
  align-items: stretch;
  gap: 16px;
}

.settings-page__item-row {
  min-height: 0;
  padding: 0;
}

.settings-page__security-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
  width: 100%;
  padding-top: 4px;
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

.settings-page__item-body :deep(.text-body-2) {
  margin-top: 4px;
  line-height: 1.55;
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
  max-width: 200px;
  flex-shrink: 0;
}

.settings-page__metric {
  flex-shrink: 0;
  min-width: 52px;
  text-align: right;
  font-size: 0.9375rem;
  font-weight: 600;
  color: rgb(var(--v-theme-primary));
}

/* ── Color palette row ── */

.settings-page__color-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  padding-left: 44px;
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
