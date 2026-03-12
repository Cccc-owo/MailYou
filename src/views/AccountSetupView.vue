<template>
  <div class="account-setup-page">
    <AppTitleBar title="MailStack" subtitle="Connect a new mailbox">
      <template #actions>
        <v-btn prepend-icon="mdi-arrow-left" @click="router.push('/')">Back to mail</v-btn>
      </template>
    </AppTitleBar>

    <v-container class="account-setup-page__content" max-width="720">
      <v-card class="pa-6">
        <div class="text-overline mb-2">Account setup</div>
        <div class="text-h4 mb-2">Connect a new mailbox</div>
        <div class="text-body-1 text-medium-emphasis mb-6">
          Configure IMAP and SMTP manually, test the connection, then save the account.
        </div>

        <v-form class="account-setup__form">
          <v-text-field v-model="draft.displayName" label="Display name" />
          <v-text-field v-model="draft.email" label="Email address" />
          <v-text-field v-model="draft.provider" label="Provider" />
          <v-row>
            <v-col cols="8">
              <v-text-field v-model="draft.incomingHost" label="Incoming host" />
            </v-col>
            <v-col cols="4">
              <v-text-field v-model.number="draft.incomingPort" label="Port" type="number" />
            </v-col>
          </v-row>
          <v-row>
            <v-col cols="8">
              <v-text-field v-model="draft.outgoingHost" label="Outgoing host" />
            </v-col>
            <v-col cols="4">
              <v-text-field v-model.number="draft.outgoingPort" label="Port" type="number" />
            </v-col>
          </v-row>
          <v-text-field v-model="draft.username" label="Username" />
          <v-text-field v-model="draft.password" label="Password / app password" type="password" />
          <v-switch v-model="draft.useTls" label="Use TLS" color="primary" />

          <div class="d-flex justify-space-between align-center flex-wrap ga-3 mt-4">
            <v-chip color="secondary">Backend now validates and persists account settings locally</v-chip>
            <div class="d-flex ga-3 flex-wrap">
              <v-btn
                prepend-icon="mdi-connection"
                :disabled="!canSave || isSaving"
                :loading="accountsStore.isTestingConnection"
                @click="runConnectionTest"
              >
                Test connection
              </v-btn>
            <v-btn
              color="primary"
              prepend-icon="mdi-check"
              :disabled="!canSave"
              :loading="isSaving"
              @click="saveAccount"
            >
              Save account
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
import { computed, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAccountsStore } from '@/stores/accounts'
import type { AccountSetupDraft } from '@/types/account'

const router = useRouter()
const accountsStore = useAccountsStore()
const isSaving = ref(false)
const error = ref<string | null>(null)

const draft = reactive<AccountSetupDraft>({
  displayName: 'New account',
  email: '',
  provider: 'IMAP / SMTP',
  incomingHost: 'imap.example.com',
  incomingPort: 993,
  outgoingHost: 'smtp.example.com',
  outgoingPort: 465,
  username: '',
  password: '',
  useTls: true,
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
    await accountsStore.createAccount({ ...draft })
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
