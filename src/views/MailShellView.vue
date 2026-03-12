<template>
  <MailShellLayout :search="messagesStore.query" :subtitle="subtitle" @update:search="messagesStore.query = $event">
    <template #actions>
      <v-btn prepend-icon="mdi-sync" :disabled="!accountsStore.currentAccountId" :loading="isSyncing" @click="syncCurrentAccount">
        Sync
      </v-btn>
      <v-btn icon="mdi-theme-light-dark" @click="uiStore.toggleAppearance" />
      <v-btn icon="mdi-cog-outline" @click="router.push('/settings')" />
    </template>

    <template #sidebar>
      <MailSidebar
        :accounts="accountsStore.accounts"
        :current-account="accountsStore.currentAccount"
        :current-account-id="accountsStore.currentAccountId"
        :current-folder-id="mailboxesStore.currentFolderId"
        :folders="mailboxesStore.folders"
        @add-account="router.push('/account-setup')"
        @compose="openComposer"
        @select-account="handleAccountChange"
        @select-folder="handleFolderChange"
      />
    </template>

    <template #list>
      <MailList
        :error="messagesStore.error"
        :is-loading="messagesStore.isLoading"
        :is-search-result="messagesStore.hasSearchQuery"
        :messages="messagesStore.filteredMessages"
        :selected-message-id="messagesStore.selectedMessageId"
        :title="mailboxesStore.currentFolder?.name ?? 'Mailbox'"
        @select-message="messagesStore.selectMessage"
        @toggle-star="toggleStar"
      />
    </template>

    <template #reader>
      <MailReader
        :has-messages="messagesStore.filteredMessages.length > 0"
        :has-search-query="messagesStore.hasSearchQuery"
        :message="messagesStore.selectedMessage"
        @archive="archiveCurrentMessage"
        @delete="deleteCurrentMessage"
        @forward="forwardCurrentMessage"
        @reply="replyToCurrentMessage"
        @toggle-read="toggleReadCurrentMessage"
      />
    </template>
  </MailShellLayout>

  <ComposerDialog
    :draft="composerStore.draft"
    :is-saving="composerStore.isSaving"
    :is-sending="composerStore.isSending"
    :model-value="composerStore.isOpen"
    @close="closeComposer"
    @save="saveDraft"
    @send="sendDraft"
    @update:draft="composerStore.draft = $event"
    @update:model-value="composerStore.isOpen = $event"
  />

  <v-snackbar
    :model-value="Boolean(composerStore.error)"
    color="error"
    location="bottom right"
    @update:model-value="!$event && composerStore.clearFeedback()"
  >
    {{ composerStore.error }}
  </v-snackbar>

  <v-snackbar
    :model-value="Boolean(composerStore.successMessage)"
    color="secondary"
    location="bottom right"
    @update:model-value="!$event && composerStore.clearFeedback()"
  >
    {{ composerStore.successMessage }}
  </v-snackbar>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import MailShellLayout from '@/layouts/MailShellLayout.vue'
import ComposerDialog from '@/components/mail/ComposerDialog.vue'
import MailList from '@/components/mail/MailList.vue'
import MailReader from '@/components/mail/MailReader.vue'
import MailSidebar from '@/components/mail/MailSidebar.vue'
import { useAccountsStore } from '@/stores/accounts'
import { useComposerStore } from '@/stores/composer'
import { useMailboxesStore } from '@/stores/mailboxes'
import { useMessagesStore } from '@/stores/messages'
import { useUiStore } from '@/stores/ui'
import { mailRepository } from '@/services/mail'

const router = useRouter()
const accountsStore = useAccountsStore()
const mailboxesStore = useMailboxesStore()
const messagesStore = useMessagesStore()
const composerStore = useComposerStore()
const uiStore = useUiStore()

const { currentAccount } = storeToRefs(accountsStore)
const { syncStatus } = storeToRefs(messagesStore)

const subtitle = computed(() => {
  if (!accountsStore.accounts.length) {
    return 'Add an account to start using MailStack'
  }

  if (!currentAccount.value) {
    return 'Choose an account to load its mailbox'
  }

  return syncStatus.value?.message ?? `${currentAccount.value.provider} · ${currentAccount.value.email}`
})

const isSyncing = computed(() => syncStatus.value?.state === 'syncing')

const loadMailbox = async (accountId: string) => {
  const bundle = await mailRepository.getMailboxBundle(accountId)

  mailboxesStore.setFolders(bundle.folders)
  messagesStore.setMailboxBundle(bundle, mailboxesStore.currentFolderId)
}

const refreshMailbox = async () => {
  if (!accountsStore.currentAccountId) {
    return
  }

  await loadMailbox(accountsStore.currentAccountId)
}

const handleAccountChange = async (accountId: string) => {
  accountsStore.selectAccount(accountId)
  await loadMailbox(accountId)
}

const handleFolderChange = async (folderId: string) => {
  if (!accountsStore.currentAccountId) {
    return
  }

  messagesStore.clearError()
  mailboxesStore.selectFolder(folderId)
  await messagesStore.loadMessages(accountsStore.currentAccountId, folderId)
}

const openComposer = () => {
  if (!accountsStore.currentAccountId) {
    return
  }

  composerStore.clearFeedback()
  composerStore.openNew(accountsStore.currentAccountId)
}

const closeComposer = () => {
  composerStore.close()
  composerStore.clearFeedback()
}

const replyToCurrentMessage = () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessage) {
    return
  }

  composerStore.clearFeedback()
  composerStore.openReply(accountsStore.currentAccountId, messagesStore.selectedMessage)
}

const forwardCurrentMessage = () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessage) {
    return
  }

  composerStore.clearFeedback()
  composerStore.openForward(accountsStore.currentAccountId, messagesStore.selectedMessage)
}

const saveDraft = async () => {
  if (!accountsStore.currentAccountId) {
    return
  }

  await composerStore.saveDraft()
  await refreshMailbox()
}

const sendDraft = async () => {
  if (!accountsStore.currentAccountId) {
    return
  }

  await composerStore.sendDraft()
  await refreshMailbox()
}

const toggleStar = async (messageId: string) => {
  if (!accountsStore.currentAccountId) {
    return
  }

  await messagesStore.toggleStar(accountsStore.currentAccountId, messageId)
}

const toggleReadCurrentMessage = async () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  await messagesStore.toggleRead(accountsStore.currentAccountId, messagesStore.selectedMessageId)
}

const deleteCurrentMessage = async () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  await messagesStore.deleteMessage(accountsStore.currentAccountId, messagesStore.selectedMessageId)
}

const archiveCurrentMessage = async () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  await messagesStore.archiveMessage(accountsStore.currentAccountId, messagesStore.selectedMessageId)
}

const syncCurrentAccount = async () => {
  if (!accountsStore.currentAccountId) {
    return
  }

  messagesStore.setSyncStatus({
    accountId: accountsStore.currentAccountId,
    state: 'syncing',
    message: 'Sync in progress…',
    updatedAt: new Date().toISOString(),
  })
  await messagesStore.syncAccount(accountsStore.currentAccountId)
  await refreshMailbox()
}

onMounted(async () => {
  await accountsStore.loadAccounts()

  if (accountsStore.currentAccountId) {
    await loadMailbox(accountsStore.currentAccountId)
  }
})
</script>
