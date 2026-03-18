<template>
  <AppScrollablePage
    title="MailYou"
    :subtitle="isEditMode ? t('accountSetup.editSubtitle') : t('accountSetup.subtitle')"
    :max-width="720"
    content-class="account-setup-page__content"
  >
    <template #actions>
      <BackActionButton :label="t('common.backToMail')" @click="router.push('/')" />
    </template>

    <v-card class="pa-6">
          <FormPageHero
            :eyebrow="t('accountSetup.title')"
            :title="isEditMode ? t('accountSetup.editHeading') : t('accountSetup.heading')"
          />

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

            <v-expansion-panels variant="accordion">
              <v-expansion-panel>
                <v-expansion-panel-title>
                  <div>
                    <div class="text-subtitle-1">{{ t('accountSetup.identitiesTitle') }}</div>
                    <div class="text-body-2 text-medium-emphasis">
                      {{ identitiesSummary }}
                    </div>
                  </div>
                </v-expansion-panel-title>
                <v-expansion-panel-text>
                  <div class="d-flex align-center justify-space-between flex-wrap ga-3 mb-4">
                    <div class="text-body-2 text-medium-emphasis">{{ t('accountSetup.identitiesHint') }}</div>
                    <v-btn variant="text" prepend-icon="mdi-plus" @click="addIdentity">
                      {{ t('accountSetup.addIdentity') }}
                    </v-btn>
                  </div>

                  <div class="account-identity-list">
                    <v-card
                      v-for="identity in draft.identities"
                      :key="identity.id"
                      variant="outlined"
                      class="pa-4"
                    >
                      <div class="d-flex align-center justify-space-between flex-wrap ga-3 mb-3">
                        <div class="text-subtitle-2">{{ identityLabel(identity) }}</div>
                        <div class="d-flex align-center ga-2 flex-wrap">
                          <v-radio-group
                            :model-value="selectedDefaultIdentityId"
                            inline
                            hide-details
                            @update:model-value="setDefaultIdentity"
                          >
                            <v-radio :label="t('accountSetup.defaultIdentity')" :value="identity.id" />
                          </v-radio-group>
                          <v-btn
                            variant="text"
                            color="error"
                            icon="mdi-delete-outline"
                            :disabled="draft.identities.length === 1"
                            @click="removeIdentity(identity.id)"
                          />
                        </div>
                      </div>

                      <v-row>
                        <v-col cols="12" md="6">
                          <v-text-field
                            :model-value="identity.name"
                            :label="t('accountSetup.identityName')"
                            @update:model-value="updateIdentityField(identity.id, 'name', String($event ?? ''))"
                          />
                        </v-col>
                        <v-col cols="12" md="6">
                          <v-text-field
                            :model-value="identity.email"
                            :label="t('accountSetup.identityEmail')"
                            @update:model-value="updateIdentityField(identity.id, 'email', String($event ?? ''))"
                          />
                        </v-col>
                      </v-row>
                      <v-text-field
                        :model-value="identity.replyTo ?? ''"
                        :label="t('accountSetup.identityReplyTo')"
                        @update:model-value="updateIdentityField(identity.id, 'replyTo', String($event ?? ''))"
                      />
                      <v-textarea
                        :model-value="identity.signature ?? ''"
                        :label="t('accountSetup.identitySignature')"
                        rows="4"
                        auto-grow
                        @update:model-value="updateIdentityField(identity.id, 'signature', String($event ?? ''))"
                      />
                    </v-card>
                  </div>
                </v-expansion-panel-text>
              </v-expansion-panel>
            </v-expansion-panels>

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
  </AppScrollablePage>
</template>

<script setup lang="ts">
import BackActionButton from '@/components/ui/BackActionButton.vue'
import AppScrollablePage from '@/components/ui/AppScrollablePage.vue'
import FormPageHero from '@/components/ui/FormPageHero.vue'
import { FALLBACK_OAUTH_PROVIDERS, MAIL_PROVIDER_PRESETS } from '@/config/mailProviders'
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { useAccountsStore } from '@/stores/accounts'
import type { AccountSetupDraft, MailIdentity } from '@/types/account'

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
  identities: [],
})

const createIdentityId = () => `identity-${crypto.randomUUID()}`

const createIdentityDraft = (overrides: Partial<MailIdentity> = {}): MailIdentity => ({
  id: createIdentityId(),
  name: draft.displayName.trim() || draft.email.trim(),
  email: draft.email.trim(),
  replyTo: null,
  signature: null,
  isDefault: draft.identities.length === 0,
  ...overrides,
})

const selectedDefaultIdentityId = computed(
  () => draft.identities.find((identity) => identity.isDefault)?.id ?? draft.identities[0]?.id ?? null,
)
const identitiesSummary = computed(() => {
  const defaultIdentity = draft.identities.find((identity) => identity.isDefault) ?? draft.identities[0]
  if (!defaultIdentity) {
    return t('accountSetup.identitiesHint')
  }

  if (draft.identities.length === 1) {
    return identityLabel(defaultIdentity)
  }

  return t('accountSetup.identitiesSummary', {
    identity: identityLabel(defaultIdentity),
    count: draft.identities.length,
  })
})

const identityLabel = (identity: MailIdentity) => {
  const email = identity.email.trim()
  const name = identity.name.trim()
  return name && email ? `${name} <${email}>` : name || email || t('accountSetup.identityUnnamed')
}

const ensureDefaultIdentity = () => {
  if (draft.identities.length === 0) {
    draft.identities.push(createIdentityDraft({ isDefault: true }))
    return
  }

  const fallbackId = selectedDefaultIdentityId.value ?? draft.identities[0].id
  draft.identities = draft.identities.map((identity) => ({
    ...identity,
    isDefault: identity.id === fallbackId,
  }))
}

const addIdentity = () => {
  draft.identities.push(createIdentityDraft({ isDefault: false }))
  ensureDefaultIdentity()
}

const removeIdentity = (identityId: string) => {
  if (draft.identities.length === 1) {
    return
  }

  draft.identities = draft.identities.filter((identity) => identity.id !== identityId)
  ensureDefaultIdentity()
}

const setDefaultIdentity = (identityId: string | null) => {
  if (!identityId) {
    return
  }

  draft.identities = draft.identities.map((identity) => ({
    ...identity,
    isDefault: identity.id === identityId,
  }))
}

const updateIdentityField = (
  identityId: string,
  field: 'name' | 'email' | 'replyTo' | 'signature',
  value: string,
) => {
  draft.identities = draft.identities.map((identity) => (
    identity.id === identityId
      ? {
          ...identity,
          [field]: value.trim().length > 0 ? value : field === 'name' || field === 'email' ? value : null,
        }
      : identity
  ))
}

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
  () => draft.displayName,
  (displayName, previousDisplayName) => {
    const defaultIdentity = draft.identities.find((identity) => identity.isDefault)
    if (
      defaultIdentity &&
      (!defaultIdentity.name.trim() || defaultIdentity.name === (previousDisplayName?.trim() || draft.email.trim()))
    ) {
      defaultIdentity.name = displayName.trim() || draft.email.trim()
    }
  },
)

watch(
  () => draft.email,
  (email, previousEmail) => {
    const normalized = email.trim()
    const defaultIdentity = draft.identities.find((identity) => identity.isDefault)
    if (
      defaultIdentity &&
      (!defaultIdentity.email.trim() || defaultIdentity.email === (previousEmail?.trim() ?? ''))
    ) {
      defaultIdentity.email = normalized
    }
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
      draft.identities = config.identities?.length ? config.identities : [createIdentityDraft()]
      ensureDefaultIdentity()
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : 'Unable to load account config'
    }
  } else if (draft.identities.length === 0) {
    draft.identities = [createIdentityDraft()]
  }
})

const canSaveBase = computed(
  () =>
    Boolean(
      isEmailValid.value &&
        draft.displayName.trim() &&
        draft.identities.some((identity) => identity.email.trim()) &&
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
    await accountsStore.testAccountConnection({ ...draft, identities: [...draft.identities] })
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
    const payload = { ...draft, identities: [...draft.identities] }
    if (isEditMode.value) {
      await accountsStore.updateAccount(editAccountId.value!, payload)
    } else {
      await accountsStore.createAccount(payload)
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
.account-setup-page__content {
  padding-block: 40px;
}

.account-setup__form {
  display: grid;
  gap: 12px;
}

.account-identity-list {
  display: grid;
  gap: 12px;
}
</style>
