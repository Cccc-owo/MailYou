<template>
  <MailShellLayout :search="messagesStore.query" :subtitle="subtitle" @update:search="messagesStore.query = $event">
    <template #actions>
      <v-btn prepend-icon="mdi-sync" :disabled="!accountsStore.currentAccountId" :loading="isSyncing" @click="syncCurrentAccount">
        {{ t('shell.sync') }}
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
        :is-folders-loading="messagesStore.isLoading || isSyncing"
        @add-account="router.push('/account-setup')"
        @compose="openComposer"
        @delete-account="handleDeleteAccount"
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
        :selected-ids="messagesStore.selectedIds"
        :title="currentFolderDisplayName"
        :folders="mailboxesStore.folders"
        :current-folder-id="mailboxesStore.currentFolderId"
        @select-message="handleSelectMessage"
        @toggle-star="toggleStar"
        @toggle-selection="messagesStore.toggleSelection"
        @select-all="messagesStore.selectAll"
        @clear-selection="messagesStore.clearSelection"
        @mark-all-read="handleMarkAllRead"
        @batch-delete="handleBatchDelete"
        @batch-archive="handleBatchArchive"
        @batch-mark-read="handleBatchMarkRead"
        @batch-mark-unread="handleBatchMarkUnread"
        @batch-move="handleBatchMove"
      />
    </template>

    <template #reader>
      <MailReader
        :has-messages="messagesStore.filteredMessages.length > 0"
        :has-search-query="messagesStore.hasSearchQuery"
        :message="messagesStore.selectedMessage"
        :folders="mailboxesStore.folders"
        :current-folder-id="mailboxesStore.currentFolderId"
        :current-folder-kind="mailboxesStore.currentFolder?.kind ?? null"
        @archive="archiveCurrentMessage"
        @restore="restoreCurrentMessage"
        @delete="promptDeleteCurrentMessage"
        @forward="forwardCurrentMessage"
        @reply="replyToCurrentMessage"
        @reply-all="replyAllToCurrentMessage"
        @toggle-read="toggleReadCurrentMessage"
        @move="moveCurrentMessage"
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

  <!-- Delete confirmation dialog -->
  <v-dialog v-model="deleteConfirmDialog" max-width="400">
    <v-card>
      <v-card-title>{{ t('shell.confirmDelete') }}</v-card-title>
      <v-card-text>{{ t('shell.confirmDeleteText') }}</v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="deleteConfirmDialog = false">{{ t('common.cancel') }}</v-btn>
        <v-btn color="error" variant="tonal" @click="confirmDeleteCurrentMessage">{{ t('common.delete') }}</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>

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

  <v-snackbar
    :model-value="Boolean(messagesStore.error)"
    color="error"
    location="bottom right"
    :timeout="-1"
    @update:model-value="!$event && messagesStore.clearError()"
  >
    {{ messagesStore.error }}
    <template #actions>
      <v-btn variant="text" @click="retryLastAction">{{ t('common.retry') }}</v-btn>
      <v-btn variant="text" @click="messagesStore.clearError()">{{ t('common.dismiss') }}</v-btn>
    </template>
  </v-snackbar>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
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

const { t } = useI18n()
const router = useRouter()
const accountsStore = useAccountsStore()
const mailboxesStore = useMailboxesStore()
const messagesStore = useMessagesStore()
const composerStore = useComposerStore()
const uiStore = useUiStore()

const { currentAccount } = storeToRefs(accountsStore)
const { syncStatus } = storeToRefs(messagesStore)

const deleteConfirmDialog = ref(false)

const currentFolderDisplayName = computed(() => {
  const folder = mailboxesStore.currentFolder
  if (!folder) return t('common.mailbox')
  return folder.kind !== 'custom' ? t(`folders.${folder.kind}`) : folder.name
})

const subtitle = computed(() => {
  if (!accountsStore.accounts.length) {
    return t('shell.addAccountHint')
  }

  if (!currentAccount.value) {
    return t('shell.chooseAccountHint')
  }

  return syncStatus.value?.message ?? `${currentAccount.value.provider} · ${currentAccount.value.email}`
})

const isSyncing = computed(() => syncStatus.value?.state === 'syncing')

const loadMailbox = async (accountId: string) => {
  messagesStore.isLoading = true

  try {
    const bundle = await mailRepository.getMailboxBundle(accountId)
    mailboxesStore.setFolders(bundle.folders)
    messagesStore.setMailboxBundle(bundle, mailboxesStore.currentFolderId)
  } finally {
    messagesStore.isLoading = false
  }
}

const refreshMailbox = async () => {
  if (!accountsStore.currentAccountId) {
    return
  }

  await loadMailbox(accountsStore.currentAccountId)
}

const retryLastAction = async () => {
  messagesStore.clearError()
  if (accountsStore.currentAccountId && mailboxesStore.currentFolderId) {
    await handleFolderChange(mailboxesStore.currentFolderId)
  } else if (accountsStore.currentAccountId) {
    await handleAccountChange(accountsStore.currentAccountId)
  }
}

const handleSelectMessage = async (messageId: string) => {
  messagesStore.selectMessage(messageId)

  if (!accountsStore.currentAccountId) {
    return
  }

  const message = messagesStore.messages.find((m) => m.id === messageId)
  if (message && !message.isRead) {
    await messagesStore.toggleRead(accountsStore.currentAccountId, messageId)
    mailboxesStore.decrementUnread(message.folderId)
  }
}

const handleAccountChange = async (accountId: string) => {
  accountsStore.selectAccount(accountId)
  messagesStore.clearSelection()
  try {
    await loadMailbox(accountId)
  } catch {
    messagesStore.error = t('shell.failedToLoadMailbox')
  }
}

const handleDeleteAccount = async (accountId: string) => {
  try {
    await accountsStore.deleteAccount(accountId)
  } catch (err) {
    console.error('Failed to delete account:', err)
  }

  mailboxesStore.setFolders([])
  messagesStore.clearAll()

  if (accountsStore.currentAccountId) {
    await loadMailbox(accountsStore.currentAccountId)
  }
}

const handleFolderChange = async (folderId: string) => {
  if (!accountsStore.currentAccountId) {
    return
  }

  messagesStore.clearError()
  messagesStore.clearSelection()
  mailboxesStore.selectFolder(folderId)
  try {
    await messagesStore.loadMessages(accountsStore.currentAccountId, folderId)
  } catch {
    messagesStore.error = t('shell.failedToLoadMessages')
  }
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

const replyAllToCurrentMessage = () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessage || !currentAccount.value) {
    return
  }

  composerStore.clearFeedback()
  composerStore.openReplyAll(
    accountsStore.currentAccountId,
    messagesStore.selectedMessage,
    currentAccount.value.email,
  )
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
  await refreshMailbox()
}

const toggleReadCurrentMessage = async () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  await messagesStore.toggleRead(accountsStore.currentAccountId, messagesStore.selectedMessageId)
  await refreshMailbox()
}

const promptDeleteCurrentMessage = () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }
  deleteConfirmDialog.value = true
}

const confirmDeleteCurrentMessage = async () => {
  deleteConfirmDialog.value = false

  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  await messagesStore.deleteMessage(accountsStore.currentAccountId, messagesStore.selectedMessageId)
  await refreshMailbox()
}

const archiveCurrentMessage = async () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  await messagesStore.archiveMessage(accountsStore.currentAccountId, messagesStore.selectedMessageId)
  await refreshMailbox()
}

const restoreCurrentMessage = async () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  await messagesStore.restoreMessage(accountsStore.currentAccountId, messagesStore.selectedMessageId)
  await refreshMailbox()
}

const moveCurrentMessage = async (folderId: string) => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  await messagesStore.moveMessage(accountsStore.currentAccountId, messagesStore.selectedMessageId, folderId)
  await refreshMailbox()
}

const handleMarkAllRead = async () => {
  if (!accountsStore.currentAccountId || !mailboxesStore.currentFolderId) {
    return
  }

  await messagesStore.markAllRead(accountsStore.currentAccountId, mailboxesStore.currentFolderId)
  await refreshMailbox()
}

// --- Batch operation handlers ---
const handleBatchDelete = async () => {
  if (!accountsStore.currentAccountId) return
  await messagesStore.batchDelete(accountsStore.currentAccountId)
  await refreshMailbox()
}

const handleBatchArchive = async () => {
  if (!accountsStore.currentAccountId) return
  await messagesStore.batchArchive(accountsStore.currentAccountId)
  await refreshMailbox()
}

const handleBatchMarkRead = async () => {
  if (!accountsStore.currentAccountId) return
  await messagesStore.batchToggleRead(accountsStore.currentAccountId, true)
  await refreshMailbox()
}

const handleBatchMarkUnread = async () => {
  if (!accountsStore.currentAccountId) return
  await messagesStore.batchToggleRead(accountsStore.currentAccountId, false)
  await refreshMailbox()
}

const handleBatchMove = async (folderId: string) => {
  if (!accountsStore.currentAccountId) return
  await messagesStore.batchMove(accountsStore.currentAccountId, folderId)
  await refreshMailbox()
}

const knownMessageIds = ref<Set<string>>(new Set())

const syncCurrentAccount = async () => {
  if (!accountsStore.currentAccountId) {
    return
  }

  messagesStore.setSyncStatus({
    accountId: accountsStore.currentAccountId,
    state: 'syncing',
    message: t('shell.syncInProgress'),
    updatedAt: new Date().toISOString(),
  })

  const oldIds = new Set(knownMessageIds.value)
  await messagesStore.syncAccount(accountsStore.currentAccountId)
  await refreshMailbox()

  // Detect new unread messages and show desktop notification
  const newUnread = messagesStore.messages.filter(
    (m) => !m.isRead && !oldIds.has(m.id),
  )
  knownMessageIds.value = new Set(messagesStore.messages.map((m) => m.id))

  if (newUnread.length > 0 && Notification.permission === 'granted') {
    if (newUnread.length === 1) {
      new Notification(newUnread[0].subject || t('shell.newMessage'), {
        body: t('shell.fromSender', { sender: newUnread[0].from }),
      })
    } else {
      new Notification(t('shell.newMail'), {
        body: t('shell.newMessagesCount', { count: newUnread.length }),
      })
    }
  }
}

onMounted(async () => {
  if ('Notification' in window && Notification.permission === 'default') {
    Notification.requestPermission()
  }

  await accountsStore.loadAccounts()

  if (accountsStore.currentAccountId) {
    await loadMailbox(accountsStore.currentAccountId)
    knownMessageIds.value = new Set(messagesStore.messages.map((m) => m.id))

    syncCurrentAccount()
  }
})

const SYNC_INTERVAL_MS = computed(() => uiStore.syncIntervalMinutes * 60 * 1000)

const handleKeyboard = (event: KeyboardEvent) => {
  const tag = (event.target as HTMLElement)?.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || (event.target as HTMLElement)?.isContentEditable) {
    return
  }

  if (event.key === 'Escape' && composerStore.isOpen) {
    closeComposer()
    return
  }

  if (composerStore.isOpen) {
    return
  }

  const msgs = messagesStore.filteredMessages
  const currentIdx = msgs.findIndex((m) => m.id === messagesStore.selectedMessageId)

  switch (event.key) {
    case 'j':
      if (currentIdx < msgs.length - 1) {
        handleSelectMessage(msgs[currentIdx + 1].id)
      }
      break
    case 'k':
      if (currentIdx > 0) {
        handleSelectMessage(msgs[currentIdx - 1].id)
      }
      break
    case 'r':
      if (event.shiftKey) {
        replyAllToCurrentMessage()
      } else {
        replyToCurrentMessage()
      }
      break
    case 'f':
      forwardCurrentMessage()
      break
    case 'e':
      archiveCurrentMessage()
      break
    case '#':
      promptDeleteCurrentMessage()
      break
    case 's':
      if (messagesStore.selectedMessageId && accountsStore.currentAccountId) {
        toggleStar(messagesStore.selectedMessageId)
      }
      break
    case 'c':
      openComposer()
      break
    case 'u':
      toggleReadCurrentMessage()
      break
  }
}

let syncInterval: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  window.addEventListener('keydown', handleKeyboard)
  syncInterval = setInterval(() => {
    if (accountsStore.currentAccountId) {
      syncCurrentAccount()
    }
  }, SYNC_INTERVAL_MS.value)
})

onUnmounted(() => {
  if (syncInterval) {
    clearInterval(syncInterval)
    syncInterval = null
  }
  window.removeEventListener('keydown', handleKeyboard)
})

watch(SYNC_INTERVAL_MS, (newMs) => {
  if (syncInterval) {
    clearInterval(syncInterval)
  }
  syncInterval = setInterval(() => {
    if (accountsStore.currentAccountId) {
      syncCurrentAccount()
    }
  }, newMs)
})
</script>
