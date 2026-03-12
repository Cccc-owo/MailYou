<template>
  <MailShellLayout :search="messagesStore.query" :subtitle="subtitle" @update:search="messagesStore.query = $event">
    <template #actions>
      <v-btn prepend-icon="mdi-sync" :loading="isSyncing" @click="syncCurrentAccount">Sync</v-btn>
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
        :is-loading="messagesStore.isLoading"
        :messages="messagesStore.filteredMessages"
        :selected-message-id="messagesStore.selectedMessageId"
        :title="mailboxesStore.currentFolder?.name ?? 'Mailbox'"
        @select-message="messagesStore.selectMessage"
        @toggle-star="toggleStar"
      />
    </template>

    <template #reader>
      <MailReader
        :message="messagesStore.selectedMessage"
        @delete="deleteCurrentMessage"
        @forward="forwardCurrentMessage"
        @reply="replyToCurrentMessage"
        @toggle-read="toggleReadCurrentMessage"
      />
    </template>
  </MailShellLayout>

  <ComposerDialog
    :draft="composerStore.draft"
    :is-sending="composerStore.isSending"
    :model-value="composerStore.isOpen"
    @close="composerStore.close"
    @save="composerStore.saveDraft"
    @send="composerStore.sendDraft"
    @update:draft="composerStore.draft = $event"
    @update:model-value="composerStore.isOpen = $event"
  />
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
  if (!currentAccount.value) {
    return 'Loading your workspace'
  }

  return syncStatus.value?.message ?? `${currentAccount.value.provider} · ${currentAccount.value.email}`
})

const isSyncing = computed(() => syncStatus.value?.state === 'syncing')

const loadMailbox = async (accountId: string) => {
  const bundle = await mailRepository.getMailboxBundle(accountId)

  mailboxesStore.setFolders(bundle.folders)
  messagesStore.setMailboxBundle(bundle, mailboxesStore.currentFolderId)
}

const handleAccountChange = async (accountId: string) => {
  accountsStore.selectAccount(accountId)
  await loadMailbox(accountId)
}

const handleFolderChange = async (folderId: string) => {
  if (!accountsStore.currentAccountId) {
    return
  }

  mailboxesStore.selectFolder(folderId)
  await messagesStore.loadMessages(accountsStore.currentAccountId, folderId)
}

const openComposer = () => {
  if (!accountsStore.currentAccountId) {
    return
  }

  composerStore.openNew(accountsStore.currentAccountId)
}

const replyToCurrentMessage = () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessage) {
    return
  }

  composerStore.openReply(accountsStore.currentAccountId, messagesStore.selectedMessage)
}

const forwardCurrentMessage = () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessage) {
    return
  }

  composerStore.openForward(accountsStore.currentAccountId, messagesStore.selectedMessage)
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
}

onMounted(async () => {
  await accountsStore.loadAccounts()

  if (accountsStore.currentAccountId) {
    await loadMailbox(accountsStore.currentAccountId)
  }
})
</script>
