<template>
  <div class="account-setup-page">
    <AppTitleBar title="MailYou" :subtitle="isEditMode ? t('accountSetup.editSubtitle') : t('accountSetup.subtitle')">
      <template #actions>
        <v-btn prepend-icon="mdi-arrow-left" @click="router.push('/')">{{ t('common.backToMail') }}</v-btn>
      </template>
    </AppTitleBar>

    <div class="account-setup-page__scroller">
      <v-container class="account-setup-page__content" max-width="720">
        <v-card class="pa-6">
          <div class="text-overline mb-2">{{ t('accountSetup.title') }}</div>
          <div class="text-h4 mb-6">{{ isEditMode ? t('accountSetup.editHeading') : t('accountSetup.heading') }}</div>

          <v-form class="account-setup__form">
            <v-text-field
              v-model="draft.email"
              :label="t('accountSetup.emailAddress')"
              :hint="t('accountSetup.emailHint')"
              persistent-hint
            />
            <v-text-field
              v-model="draft.displayName"
              :label="t('accountSetup.displayName')"
              :hint="t('accountSetup.displayNameHint')"
              persistent-hint
            />

            <template v-if="showAuthSection">
              <v-select
                v-if="authModeOptions.length > 1"
                v-model="draft.authMode"
                :items="authModeOptions"
                :label="t('accountSetup.authMode')"
                item-title="label"
                item-value="value"
                density="compact"
              />

              <div class="d-flex align-center justify-space-between flex-wrap ga-3">
                <div class="text-body-2 text-medium-emphasis">
                  {{ providerSummary }}
                </div>
                <v-switch
                  v-model="advancedMode"
                  hide-details
                  inset
                  color="primary"
                  :label="t('accountSetup.advancedMode')"
                />
              </div>

              <template v-if="isOAuthMode">
                <v-select
                  v-model="draft.oauthProvider"
                  :items="oauthProviderOptions"
                  :label="t('accountSetup.oauthProvider')"
                  item-title="label"
                  item-value="value"
                  density="compact"
                  :readonly="isProviderLocked"
                />
                <v-select
                  v-model="draft.oauthSource"
                  :items="oauthSourceOptions"
                  :label="t('accountSetup.oauthSource')"
                  item-title="label"
                  item-value="value"
                  density="compact"
                />
                <v-alert type="info" variant="tonal">
                  {{ t('accountSetup.oauthProviderHint') }}
                </v-alert>
                <v-alert
                  v-if="draft.oauthSource === 'direct' && selectedOAuthProvider && !selectedOAuthProvider.supportsDirect"
                  type="warning"
                  variant="tonal"
                >
                  {{ t('accountSetup.oauthDirectUnavailable') }}
                </v-alert>
                <div class="d-flex align-center flex-wrap ga-3">
                  <v-btn
                    color="primary"
                    prepend-icon="mdi-shield-key-outline"
                    :disabled="!canAuthorizeOAuth || isAuthorizingOAuth"
                    :loading="isAuthorizingOAuth"
                    @click="runOAuthAuthorization"
                  >
                    {{ oauthAuthorizeLabel }}
                  </v-btn>
                  <div v-if="isOAuthAuthorized" class="text-body-2 text-medium-emphasis">
                    {{ t('accountSetup.oauthAuthorizedUntil', { time: formattedTokenExpiry }) }}
                  </div>
                </div>
              </template>

              <template v-if="showAdvancedConnectionFields">
                <v-select
                  v-model="draft.incomingProtocol"
                  :items="protocolOptions"
                  :label="t('accountSetup.incomingProtocol')"
                  item-title="label"
                  item-value="value"
                  density="compact"
                  :disabled="isOAuthMode"
                />

                <v-row>
                  <v-col cols="8">
                    <v-text-field v-model="draft.incomingHost" :label="t('accountSetup.incomingHost')" />
                  </v-col>
                  <v-col cols="4">
                    <v-text-field v-model.number="draft.incomingPort" :label="t('accountSetup.port')" type="number" />
                  </v-col>
                </v-row>
                <v-row>
                  <v-col cols="8">
                    <v-text-field v-model="draft.outgoingHost" :label="t('accountSetup.outgoingHost')" />
                  </v-col>
                  <v-col cols="4">
                    <v-text-field v-model.number="draft.outgoingPort" :label="t('accountSetup.port')" type="number" />
                  </v-col>
                </v-row>
                <v-switch v-model="draft.useTls" :label="t('accountSetup.useTls')" color="primary" />
              </template>

              <v-alert v-else type="info" variant="tonal">
                {{ t('accountSetup.serverPresetApplied') }}
              </v-alert>

              <v-text-field
                v-model="draft.username"
                :label="t('accountSetup.username')"
                :readonly="isOAuthMode && !advancedMode"
              />
              <v-text-field
                v-if="!isOAuthMode"
                v-model="draft.password"
                :label="t('accountSetup.password')"
                type="password"
              />

              <div class="d-flex justify-space-between align-center flex-wrap ga-3 mt-4">
                <div class="d-flex ga-3 flex-wrap">
                  <v-btn
                    prepend-icon="mdi-connection"
                    :disabled="!canTestConnection || isSaving"
                    :loading="accountsStore.isTestingConnection"
                    @click="runConnectionTest"
                  >
                    {{ t('accountSetup.testConnection') }}
                  </v-btn>
                  <v-btn
                    color="primary"
                    prepend-icon="mdi-check"
                    :disabled="!canSave"
                    :loading="isSaving"
                    @click="saveAccount"
                  >
                    {{ isEditMode ? t('accountSetup.updateAccount') : t('accountSetup.saveAccount') }}
                  </v-btn>
                </div>
              </div>
            </template>

            <v-alert v-else type="info" variant="tonal">
              {{ t('accountSetup.enterValidEmail') }}
            </v-alert>

            <v-alert
              v-if="accountsStore.connectionStatus"
              class="mt-4"
              type="success"
              variant="tonal"
            >
              {{ accountsStore.connectionStatus.message }}
            </v-alert>
            <v-alert v-if="error" class="mt-4" type="error" variant="tonal">{{ error }}</v-alert>
          </v-form>
        </v-card>
      </v-container>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppTitleBar from '@/components/AppTitleBar.vue'
import { FALLBACK_OAUTH_PROVIDERS, MAIL_PROVIDER_PRESETS } from '@/config/mailProviders'
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { useAccountsStore } from '@/stores/accounts'
import type { AccountSetupDraft } from '@/types/account'

const emailPattern = /^[^\s@]+@[^\s@]+\.[^\s@]+$/

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const accountsStore = useAccountsStore()
const isSaving = ref(false)
const isAuthorizingOAuth = ref(false)
const advancedMode = ref(false)
const error = ref<string | null>(null)

const editAccountId = computed(() => route.params.accountId as string | undefined)
const isEditMode = computed(() => Boolean(editAccountId.value))
const draft = reactive<AccountSetupDraft>({
  displayName: '',
  email: '',
  provider: 'IMAP / SMTP',
  authMode: 'password',
  incomingProtocol: 'imap',
  incomingHost: 'imap.example.com',
  incomingPort: 993,
  outgoingHost: 'smtp.example.com',
  outgoingPort: 465,
  username: '',
  password: '',
  useTls: true,
  oauthProvider: null,
  oauthSource: null,
  accessToken: '',
  refreshToken: '',
  tokenExpiresAt: '',
})

const protocolOptions = computed(() => [
  { label: t('accountSetup.protocolImap'), value: 'imap' },
  { label: t('accountSetup.protocolPop3'), value: 'pop3' },
])

const authModeOptions = computed(() => {
  if (isEditMode.value && draft.authMode === 'oauth') {
    return [
      { label: t('accountSetup.authModePassword'), value: 'password' },
      { label: t('accountSetup.authModeOauth'), value: 'oauth' },
    ]
  }

  if (activePreset.value?.authMode === 'oauth') {
    return [
      { label: t('accountSetup.authModePassword'), value: 'password' },
      { label: t('accountSetup.authModeOauth'), value: 'oauth' },
    ]
  }

  return [{ label: t('accountSetup.authModePassword'), value: 'password' }]
})

const normalizedEmail = computed(() => draft.email.trim().toLowerCase())
const emailDomain = computed(() => normalizedEmail.value.split('@')[1] ?? '')
const isEmailValid = computed(() => emailPattern.test(normalizedEmail.value))
const showAuthSection = computed(() => isEditMode.value || isEmailValid.value)
const activePreset = computed(() => MAIL_PROVIDER_PRESETS[emailDomain.value] ?? null)
const hasProviderPreset = computed(() => Boolean(activePreset.value))
const isOAuthMode = computed(() => draft.authMode === 'oauth')
const oauthProviderCatalog = computed(() =>
  accountsStore.oauthProviders.length > 0 ? accountsStore.oauthProviders : FALLBACK_OAUTH_PROVIDERS,
)
const selectedOAuthProvider = computed(() =>
  oauthProviderCatalog.value.find((provider) => provider.id === draft.oauthProvider) ?? null,
)
const oauthProviderOptions = computed(() =>
  oauthProviderCatalog.value.map((provider) => ({
    label: provider.label,
    value: provider.id,
  })),
)
const oauthSourceOptions = computed(() => {
  const provider = selectedOAuthProvider.value
  const options = []

  if (!provider || provider.supportsDirect) {
    options.push({ label: t('accountSetup.oauthSourceDirect'), value: 'direct' })
  }

  if (!provider || provider.supportsProxy) {
    options.push({ label: t('accountSetup.oauthSourceProxy'), value: 'proxy' })
  }

  return options
})
const isOAuthAuthorized = computed(() => Boolean(draft.accessToken.trim()))
const formattedTokenExpiry = computed(() => {
  if (!draft.tokenExpiresAt.trim()) {
    return ''
  }

  const date = new Date(draft.tokenExpiresAt)
  if (Number.isNaN(date.getTime())) {
    return draft.tokenExpiresAt
  }

  return date.toLocaleString()
})
const oauthAuthorizeLabel = computed(() =>
  isOAuthAuthorized.value ? t('accountSetup.reauthorizeOAuth') : t('accountSetup.authorizeOAuth'),
)
const showAdvancedConnectionFields = computed(() => advancedMode.value || !hasProviderPreset.value)
const isProviderLocked = computed(() => !advancedMode.value && Boolean(activePreset.value?.oauthProvider))
const providerSummary = computed(() => {
  if (!activePreset.value) {
    return t('accountSetup.customProviderSummary')
  }

  return t('accountSetup.detectedProviderSummary', {
    provider: activePreset.value.provider,
    mode: activePreset.value.authMode === 'oauth'
      ? t('accountSetup.authModeOauth')
      : t('accountSetup.authModePassword'),
  })
})

const applyProviderPreset = (key: string, preserveUserAuthSelection = false) => {
  const preset = MAIL_PROVIDER_PRESETS[key]
  if (!preset) {
    return
  }

  draft.provider = preset.provider
  draft.incomingHost = preset.incomingHost
  draft.incomingPort = preset.incomingPort
  draft.outgoingHost = preset.outgoingHost
  draft.outgoingPort = preset.outgoingPort
  draft.useTls = preset.useTls

  if (!preserveUserAuthSelection) {
    draft.authMode = preset.authMode
  }

  if (preset.authMode === 'oauth') {
    draft.incomingProtocol = 'imap'
    draft.oauthProvider = preset.oauthProvider ?? draft.oauthProvider
    draft.oauthSource = selectedOAuthProvider.value?.supportsDirect ? 'direct' : 'proxy'
    draft.username = draft.email.trim()
  } else {
    draft.oauthProvider = null
    draft.oauthSource = null
    if (!draft.username.trim()) {
      draft.username = draft.email.trim()
    }
  }
}

watch(
  () => draft.email,
  (email, previousEmail) => {
    if (isEditMode.value) {
      return
    }

    const trimmed = email.trim()
    if (!draft.displayName.trim() || draft.displayName === previousEmail) {
      draft.displayName = trimmed
    }

    draft.username = trimmed

    if (!emailPattern.test(trimmed)) {
      draft.provider = 'IMAP / SMTP'
      draft.authMode = 'password'
      draft.oauthProvider = null
      draft.oauthSource = null
      return
    }

    const domain = trimmed.split('@')[1]?.toLowerCase()
    if (!domain) {
      return
    }

    if (MAIL_PROVIDER_PRESETS[domain]) {
      applyProviderPreset(domain)
      return
    }

    draft.provider = domain
    draft.authMode = 'password'
    draft.oauthProvider = null
    draft.oauthSource = null
    draft.accessToken = ''
    draft.refreshToken = ''
    draft.tokenExpiresAt = ''
  },
)

watch(
  () => draft.incomingProtocol,
  (protocol) => {
    if (isEditMode.value || hasProviderPreset.value || isOAuthMode.value) {
      return
    }

    if (protocol === 'pop3') {
      draft.incomingHost = draft.incomingHost.replace('imap.', 'pop.')
      draft.incomingPort = 995
    } else if (protocol === 'imap') {
      draft.incomingHost = draft.incomingHost.replace('pop.', 'imap.')
      draft.incomingPort = 993
    }
  },
)

watch(
  () => draft.authMode,
  (authMode) => {
    if (authMode === 'oauth') {
      draft.incomingProtocol = 'imap'
      draft.oauthProvider ||= activePreset.value?.oauthProvider ?? 'gmail'
      draft.oauthSource ||= selectedOAuthProvider.value?.supportsDirect ? 'direct' : 'proxy'
      draft.username = draft.email.trim()
      return
    }

    draft.oauthProvider = null
    draft.oauthSource = null
    draft.accessToken = ''
    draft.refreshToken = ''
    draft.tokenExpiresAt = ''
  },
)

watch(
  () => draft.oauthProvider,
  (provider) => {
    if (!provider) {
      draft.provider = 'OAuth 2.0'
      return
    }

    const providerInfo = oauthProviderCatalog.value.find((item) => item.id === provider)
    draft.provider = providerInfo?.label ?? provider

    if (activePreset.value?.oauthProvider === provider) {
      applyProviderPreset(emailDomain.value, true)
    }

    if (draft.oauthSource === 'direct' && providerInfo && !providerInfo.supportsDirect) {
      draft.oauthSource = providerInfo.supportsProxy ? 'proxy' : null
    }
  },
)

onMounted(async () => {
  try {
    await accountsStore.loadOAuthProviders()
  } catch {
    // Keep the static fallback list when provider discovery is unavailable.
  }

  if (editAccountId.value) {
    try {
      const config = await accountsStore.getAccountConfig(editAccountId.value)
      Object.assign(draft, config)
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : 'Unable to load account config'
    }
  }
})

const canSaveBase = computed(
  () =>
    Boolean(
      isEmailValid.value &&
        draft.displayName.trim() &&
        draft.username.trim() &&
        draft.incomingHost.trim() &&
        draft.outgoingHost.trim(),
    ),
)

const canAuthorizeOAuth = computed(() => {
  if (!isOAuthMode.value) {
    return false
  }

  if (!isEmailValid.value || !draft.oauthProvider || !draft.oauthSource) {
    return false
  }

  if (draft.oauthSource === 'direct' && selectedOAuthProvider.value && !selectedOAuthProvider.value.supportsDirect) {
    return false
  }

  return true
})

const canSave = computed(() =>
  canSaveBase.value && (!isOAuthMode.value || isOAuthAuthorized.value),
)
const canTestConnection = computed(() =>
  canSaveBase.value && (!isOAuthMode.value || isOAuthAuthorized.value),
)

const runOAuthAuthorization = async () => {
  if (!canAuthorizeOAuth.value || isAuthorizingOAuth.value) {
    return
  }

  isAuthorizingOAuth.value = true
  error.value = null

  try {
    draft.username = draft.email.trim()
    const result = await accountsStore.authorizeOAuth({
      provider: draft.oauthProvider!,
      source: draft.oauthSource!,
    })
    draft.accessToken = result.accessToken
    draft.refreshToken = result.refreshToken
    draft.tokenExpiresAt = result.expiresAt
  } catch (authorizeError) {
    error.value = authorizeError instanceof Error ? authorizeError.message : 'Unable to authorize OAuth account'
  } finally {
    isAuthorizingOAuth.value = false
  }
}

const runConnectionTest = async () => {
  if (!canTestConnection.value || isSaving.value) {
    return
  }

  error.value = null

  try {
    await accountsStore.testAccountConnection({ ...draft })
  } catch (testError) {
    error.value = testError instanceof Error ? testError.message : 'Unable to test account connection'
  }
}

const saveAccount = async () => {
  if (!canSave.value || isSaving.value) {
    return
  }

  isSaving.value = true
  error.value = null

  try {
    if (isEditMode.value) {
      await accountsStore.updateAccount(editAccountId.value!, { ...draft })
    } else {
      await accountsStore.createAccount({ ...draft })
    }
    await router.push('/')
  } catch (saveError) {
    error.value = saveError instanceof Error ? saveError.message : 'Unable to save account'
  } finally {
    isSaving.value = false
  }
}
</script>

<style scoped>
.account-setup-page {
  height: 100vh;
  background: rgb(var(--v-theme-background));
  overflow: hidden;
}

.account-setup-page__scroller {
  height: calc(100vh - 40px);
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(var(--v-theme-on-surface), 0.2) transparent;
}

.account-setup-page__scroller::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

.account-setup-page__scroller::-webkit-scrollbar-track {
  background: transparent;
}

.account-setup-page__scroller::-webkit-scrollbar-thumb {
  background: rgba(var(--v-theme-on-surface), 0.15);
  border-radius: 3px;
}

.account-setup-page__scroller::-webkit-scrollbar-thumb:hover {
  background: rgba(var(--v-theme-on-surface), 0.3);
}

.account-setup-page__content {
  padding-block: 40px;
}

.account-setup__form {
  display: grid;
  gap: 12px;
}
</style>
