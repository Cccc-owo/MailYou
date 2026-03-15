<template>
  <div class="account-setup-page">
    <AppTitleBar title="MailYou" :subtitle="isEditMode ? t('accountSetup.editSubtitle') : t('accountSetup.subtitle')">
      <template #actions>
        <v-btn prepend-icon="mdi-arrow-left" @click="router.push('/')">{{ t('common.backToMail') }}</v-btn>
      </template>
    </AppTitleBar>

    <v-container class="account-setup-page__content" max-width="720">
      <v-card class="pa-6">
        <div class="text-overline mb-2">{{ t('accountSetup.title') }}</div>
        <div class="text-h4 mb-6">{{ isEditMode ? t('accountSetup.editHeading') : t('accountSetup.heading') }}</div>

        <v-form class="account-setup__form">
          <v-text-field v-model="draft.displayName" :label="t('accountSetup.displayName')" />
          <v-text-field v-model="draft.email" :label="t('accountSetup.emailAddress')" />

          <v-select
            v-model="draft.incomingProtocol"
            :items="protocolOptions"
            :label="t('accountSetup.incomingProtocol')"
            item-title="label"
            item-value="value"
            density="compact"
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
          <v-text-field v-model="draft.username" :label="t('accountSetup.username')" />
          <v-text-field v-model="draft.password" :label="t('accountSetup.password')" type="password" />
          <v-switch v-model="draft.useTls" :label="t('accountSetup.useTls')" color="primary" />

          <div class="d-flex justify-space-between align-center flex-wrap ga-3 mt-4">
            <div class="d-flex ga-3 flex-wrap">
              <v-btn
                prepend-icon="mdi-connection"
                :disabled="!canSave || isSaving"
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
</template>

<script setup lang="ts">
import AppTitleBar from '@/components/AppTitleBar.vue'
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { useAccountsStore } from '@/stores/accounts'
import type { AccountSetupDraft } from '@/types/account'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const accountsStore = useAccountsStore()
const isSaving = ref(false)
const error = ref<string | null>(null)

const editAccountId = computed(() => route.params.accountId as string | undefined)
const isEditMode = computed(() => Boolean(editAccountId.value))

const draft = reactive<AccountSetupDraft>({
  displayName: 'New account',
  email: '',
  provider: 'IMAP / SMTP',
  incomingProtocol: 'imap',
  incomingHost: 'imap.example.com',
  incomingPort: 993,
  outgoingHost: 'smtp.example.com',
  outgoingPort: 465,
  username: '',
  password: '',
  useTls: true,
})

const protocolOptions = computed(() => [
  { label: t('accountSetup.protocolImap'), value: 'imap' },
  { label: t('accountSetup.protocolPop3'), value: 'pop3' },
])

// ── Provider autoconfig ──

interface ProviderPreset {
  provider: string
  incomingHost: string
  incomingPort: number
  outgoingHost: string
  outgoingPort: number
  useTls: boolean
}

const providerPresets: Record<string, ProviderPreset> = {
  'qq.com': { provider: 'QQ', incomingHost: 'imap.qq.com', incomingPort: 993, outgoingHost: 'smtp.qq.com', outgoingPort: 465, useTls: true },
  'foxmail.com': { provider: 'QQ', incomingHost: 'imap.qq.com', incomingPort: 993, outgoingHost: 'smtp.qq.com', outgoingPort: 465, useTls: true },
}

watch(
  () => draft.email,
  (email) => {
    if (isEditMode.value) return
    const domain = email.split('@')[1]?.toLowerCase()
    if (!domain) return
    const preset = providerPresets[domain]
    if (preset) {
      Object.assign(draft, preset)
      draft.username = email
    }
  },
)

watch(
  () => draft.incomingProtocol,
  (protocol) => {
    if (isEditMode.value) return
    if (protocol === 'pop3') {
      draft.incomingHost = draft.incomingHost.replace('imap.', 'pop.')
      draft.incomingPort = 995
    } else if (protocol === 'imap') {
      draft.incomingHost = draft.incomingHost.replace('pop.', 'imap.')
      draft.incomingPort = 993
    }
  },
)

onMounted(async () => {
  if (editAccountId.value) {
    try {
      const config = await accountsStore.getAccountConfig(editAccountId.value)
      Object.assign(draft, config)
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : 'Unable to load account config'
    }
  }
})

const canSave = computed(
  () =>
    Boolean(
      draft.email.trim() &&
        draft.displayName.trim() &&
        draft.username.trim() &&
        draft.incomingHost.trim() &&
        draft.outgoingHost.trim(),
    ),
)

const runConnectionTest = async () => {
  if (!canSave.value || isSaving.value) {
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
  min-height: 100vh;
  background: rgb(var(--v-theme-background));
}

.account-setup-page__content {
  padding-block: 40px;
}

.account-setup__form {
  display: grid;
  gap: 12px;
}
</style>
