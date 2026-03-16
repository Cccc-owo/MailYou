<template>
  <MailShellLayout :search="messagesStore.query" @update:search="handleSearchUpdate">
    <template #actions>
      <v-tooltip :text="t('shell.sync')" location="bottom">
        <template #activator="{ props: tip }">
          <v-btn v-bind="tip" prepend-icon="mdi-sync" :disabled="!accountsStore.currentAccountId" :loading="isSyncing" @click="syncCurrentAccount">
            {{ t('shell.sync') }}
          </v-btn>
        </template>
      </v-tooltip>
      <v-tooltip :text="t('shell.toggleTheme')" location="bottom">
        <template #activator="{ props: tip }">
          <v-btn v-bind="tip" icon="mdi-theme-light-dark" @click="uiStore.toggleAppearance" />
        </template>
      </v-tooltip>
      <v-tooltip :text="t('contacts.title')" location="bottom">
        <template #activator="{ props: tip }">
          <v-btn v-bind="tip" icon="mdi-contacts-outline" @click="router.push('/contacts')" />
        </template>
      </v-tooltip>
      <v-tooltip :text="t('shell.settings')" location="bottom">
        <template #activator="{ props: tip }">
          <v-btn v-bind="tip" icon="mdi-cog-outline" @click="router.push('/settings')" />
        </template>
      </v-tooltip>
    </template>

    <template #sidebar>
      <MailSidebar
        :accounts="accountsStore.accounts"
        :current-account="accountsStore.currentAccount"
        :current-account-id="accountsStore.currentAccountId"
        :current-folder-id="mailboxesStore.currentFolderId"
        :folders="mailboxesStore.folders"
        :is-folders-loading="isSyncing"
        @add-account="router.push('/account-setup')"
        @compose="openComposer"
        @delete-account="handleDeleteAccount"
        @edit-account="(id) => router.push('/account-setup/' + id)"
        @select-account="handleAccountChange"
        @select-folder="handleFolderChange"
        @sync-account="handleSyncAccount"
        @mark-folder-read="handleMarkFolderRead"
      />
    </template>

    <template #list>
      <MailList
        :error="messagesStore.error"
        :is-loading="messagesStore.isLoading"
        :is-search-result="messagesStore.hasSearchQuery"
        :threads="messagesStore.threadSummaries"
        :selected-message-id="messagesStore.selectedMessageId"
        :selected-thread-id="messagesStore.selectedMessage?.threadId ?? null"
        :is-pop3="accountsStore.isCurrentAccountPop3"
        :selected-ids="messagesStore.selectedIds"
        :title="currentFolderDisplayName"
        :folders="mailboxesStore.folders"
        :current-folder-id="mailboxesStore.currentFolderId"
        :current-folder-kind="mailboxesStore.currentFolder?.kind ?? null"
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
        @context-reply="handleContextReply"
        @context-reply-all="handleContextReplyAll"
        @context-forward="handleContextForward"
        @context-toggle-read="handleContextToggleRead"
        @context-archive="handleContextArchive"
        @context-mark-spam="handleContextMarkSpam"
        @context-restore="handleContextRestore"
        @context-delete="handleContextDelete"
        @context-move="handleContextMove"
      />
    </template>

    <template #reader>
      <MailReader
        :has-messages="messagesStore.threadSummaries.length > 0"
        :has-search-query="messagesStore.hasSearchQuery"
        :message="messagesStore.selectedMessage"
        :thread-messages="messagesStore.selectedThreadMessages"
        :folders="mailboxesStore.folders"
        :current-folder-id="mailboxesStore.currentFolderId"
        :current-folder-kind="mailboxesStore.currentFolder?.kind ?? null"
        :is-pop3="accountsStore.isCurrentAccountPop3"
        @archive="archiveCurrentMessage"
        @mark-spam="markCurrentMessageSpam"
        @restore="restoreCurrentMessage"
        @delete="promptDeleteCurrentMessage"
        @forward="forwardCurrentMessage"
        @reply="replyToCurrentMessage"
        @reply-all="replyAllToCurrentMessage"
        @toggle-read="toggleReadCurrentMessage"
        @move="moveCurrentMessage"
        @save-contact="handleSaveContact"
        @compose-to="handleComposeTo"
        @view-contact="handleViewContact"
        @toggle-star="toggleStarCurrentMessage"
        @export-pdf="exportCurrentMessagePdf"
        @select-thread-message="handleSelectMessage"
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

  <!-- Draft recovery dialog -->
  <v-dialog v-model="composerStore.showRecoveryDialog" max-width="420" persistent>
    <v-card>
      <v-card-title>{{ t('composer.recovery.title') }}</v-card-title>
      <v-card-text>{{ t('composer.recovery.message') }}</v-card-text>
      <v-card-actions>
        <v-btn variant="text" @click="composerStore.cancelRecovery()">{{ t('common.cancel') }}</v-btn>
        <v-spacer />
        <v-btn variant="text" @click="composerStore.discardAndProceed()">{{ t('composer.recovery.discard') }}</v-btn>
        <v-btn color="primary" variant="tonal" @click="composerStore.recoverDraft()">{{ t('composer.recovery.restore') }}</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>

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
    class="mail-shell__snackbar mail-shell__snackbar--error"
    location="bottom right"
    @update:model-value="!$event && composerStore.clearFeedback()"
    @contextmenu="openSnackbarMenu"
  >
    <span class="mail-shell__snackbar-text">{{ composerStore.error }}</span>
    <template #actions>
      <v-btn variant="text" size="small" @click="retrySend">{{ t('common.retry') }}</v-btn>
      <v-btn variant="text" size="small" icon="mdi-close" @click="composerStore.clearFeedback()" />
    </template>
  </v-snackbar>

  <v-snackbar
    :model-value="Boolean(composerStore.successMessage)"
    color="surface-variant"
    location="bottom right"
    @update:model-value="!$event && composerStore.clearFeedback()"
  >
    {{ composerStore.successMessage }}
  </v-snackbar>

  <v-snackbar
    :model-value="Boolean(messagesStore.error)"
    class="mail-shell__snackbar mail-shell__snackbar--error"
    location="bottom right"
    :timeout="-1"
    @update:model-value="!$event && messagesStore.clearError()"
    @contextmenu="openSnackbarMenu"
  >
    <span class="mail-shell__snackbar-text">{{ messagesStore.error }}</span>
    <template #actions>
      <v-btn variant="text" @click="retryLastAction">{{ t('common.retry') }}</v-btn>
      <v-btn variant="text" @click="messagesStore.clearError()">{{ t('common.dismiss') }}</v-btn>
    </template>
  </v-snackbar>

  <!-- Snackbar right-click context menu -->
  <ContextMenu v-model="snackbarCtx.isOpen.value" :x="snackbarCtx.x.value" :y="snackbarCtx.y.value">
    <v-list-item v-if="snackbarHasSelection" prepend-icon="mdi-content-copy" :title="t('reader.copy')" @click="snackbarCopy" />
    <v-divider v-if="snackbarHasSelection" />
    <v-list-item prepend-icon="mdi-select-all" :title="t('reader.selectAll')" @click="snackbarSelectAll" />
  </ContextMenu>

  <v-snackbar
    :model-value="Boolean(undoableAction)"
    :timeout="-1"
    location="bottom right"
  >
    {{ undoableAction?.label }}
    <template #actions>
      <v-btn variant="text" @click="handleUndo">{{ t('common.undo') }}</v-btn>
      <v-btn variant="text" @click="dismissUndo">{{ t('common.dismiss') }}</v-btn>
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
import ContextMenu from '@/components/ContextMenu.vue'
import { useContextMenu } from '@/composables/useContextMenu'
import DOMPurify from 'dompurify'
import { useAccountsStore } from '@/stores/accounts'
import { useComposerStore } from '@/stores/composer'
import { useMailboxesStore } from '@/stores/mailboxes'
import { useMessagesStore } from '@/stores/messages'
import { useUiStore } from '@/stores/ui'
import { useContactsStore } from '@/stores/contacts'
import { mailRepository } from '@/services/mail'

const { t, locale } = useI18n()
const router = useRouter()
const accountsStore = useAccountsStore()
const mailboxesStore = useMailboxesStore()
const messagesStore = useMessagesStore()
const composerStore = useComposerStore()
const uiStore = useUiStore()
const contactsStore = useContactsStore()
const snackbarCtx = useContextMenu()
const snackbarHasSelection = ref(false)

const openSnackbarMenu = (event: MouseEvent) => {
  snackbarHasSelection.value = Boolean(window.getSelection()?.toString())
  snackbarCtx.open(event)
}

const snackbarSelectAll = () => {
  const sel = window.getSelection()
  if (!sel) return
  const snackbar = document.querySelector('.mail-shell__snackbar--error .v-snackbar__wrapper')
  if (!snackbar) return
  const range = document.createRange()
  range.selectNodeContents(snackbar)
  sel.removeAllRanges()
  sel.addRange(range)
}

const snackbarCopy = () => {
  const sel = window.getSelection()
  if (sel && sel.toString()) {
    navigator.clipboard.writeText(sel.toString())
  } else {
    const text = messagesStore.error || composerStore.error || ''
    navigator.clipboard.writeText(text)
  }
}

const { currentAccount } = storeToRefs(accountsStore)
const { syncStatus } = storeToRefs(messagesStore)

const deleteConfirmDialog = ref(false)
const lastFailedAction = ref<(() => Promise<void>) | null>(null)

interface UndoableAction {
  label: string
  undo: () => Promise<void>
  timer: ReturnType<typeof setTimeout>
}
const undoableAction = ref<UndoableAction | null>(null)

const performUndoable = (label: string, undoFn: () => Promise<void>) => {
  if (undoableAction.value) clearTimeout(undoableAction.value.timer)
  const timer = setTimeout(() => { undoableAction.value = null }, 5000)
  undoableAction.value = { label, undo: undoFn, timer }
}

const handleUndo = async () => {
  if (!undoableAction.value) return
  clearTimeout(undoableAction.value.timer)
  const action = undoableAction.value
  undoableAction.value = null
  await action.undo()
}

const dismissUndo = () => {
  if (undoableAction.value) {
    clearTimeout(undoableAction.value.timer)
    undoableAction.value = null
  }
}

const currentFolderDisplayName = computed(() => {
  if (messagesStore.hasSearchQuery) {
    return t('mailList.searchResultsTitle')
  }
  const folder = mailboxesStore.currentFolder
  if (!folder) return t('common.mailbox')
  return folder.kind !== 'custom' ? t(`folders.${folder.kind}`) : folder.name
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
  if (messagesStore.hasSearchQuery) {
    await messagesStore.searchMessages(accountsStore.currentAccountId, messagesStore.query)
  }
}

const handleSearchUpdate = (value: string) => {
  messagesStore.query = value
}

const retryLastAction = async () => {
  messagesStore.clearError()
  if (lastFailedAction.value) {
    const action = lastFailedAction.value
    lastFailedAction.value = null
    await action()
  } else if (accountsStore.currentAccountId && mailboxesStore.currentFolderId) {
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
  messagesStore.clearError()
  lastFailedAction.value = null
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
  messagesStore.clearError()
  lastFailedAction.value = null

  if (accountsStore.currentAccountId) {
    await loadMailbox(accountsStore.currentAccountId)
  }
}

const handleFolderChange = async (folderId: string) => {
  if (!accountsStore.currentAccountId) {
    return
  }

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

const retrySend = async () => {
  composerStore.clearFeedback()
  await sendDraft()
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

const toggleStarCurrentMessage = async () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  await toggleStar(messagesStore.selectedMessageId)
}

const buildPrintHtml = (subject: string, from: string, to: string[], cc: string[], date: string, body: string) => {
  const toLine = to.join(', ')
  const ccLine = cc.length > 0 ? `<p style="margin:2px 0;color:#555"><strong>${t('reader.ccLabel')}</strong>${cc.join(', ')}</p>` : ''
  return `<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>${subject}</title>
<style>
  body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 0 auto; padding: 32px; color: #222; }
  h1 { font-size: 1.3rem; margin: 0 0 12px; }
  .meta { border-bottom: 1px solid #ddd; padding-bottom: 12px; margin-bottom: 16px; font-size: 0.85rem; color: #555; }
  .meta p { margin: 2px 0; }
  .body { line-height: 1.7; }
  .body img { max-width: 100%; }
</style></head><body>
<h1>${subject}</h1>
<div class="meta">
  <p><strong>${t('reader.from', 'From')}: </strong>${from}</p>
  <p><strong>${t('reader.to')}</strong>${toLine}</p>
  ${ccLine}
  <p style="margin:2px 0;color:#555"><strong>${t('reader.date', 'Date')}: </strong>${date}</p>
</div>
<div class="body">${body}</div>
</body></html>`
}

const exportCurrentMessagePdf = () => {
  const msg = messagesStore.selectedMessage
  if (!msg) return
  const date = new Intl.DateTimeFormat(locale.value, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  }).format(new Date(msg.receivedAt))
  const sanitizedBody = DOMPurify.sanitize(msg.body, {
    ALLOWED_TAGS: [
      'p', 'br', 'a', 'img', 'table', 'thead', 'tbody', 'tr', 'td', 'th',
      'div', 'span', 'strong', 'b', 'em', 'i', 'u', 'ul', 'ol', 'li',
      'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'blockquote', 'pre', 'code',
      'hr', 'sub', 'sup', 'center', 'font', 'small',
    ],
    ALLOWED_ATTR: [
      'href', 'src', 'alt', 'title', 'width', 'height', 'style', 'class',
      'align', 'valign', 'border', 'cellpadding', 'cellspacing', 'bgcolor',
      'color', 'size', 'face', 'colspan', 'rowspan',
    ],
    ALLOW_DATA_ATTR: false,
  })
  const html = buildPrintHtml(msg.subject, `${msg.from} <${msg.fromEmail}>`, msg.to, msg.cc, date, sanitizedBody)
  window.windowControls?.exportPdf(html, msg.subject)
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

  const accountId = accountsStore.currentAccountId
  const messageId = messagesStore.selectedMessageId
  await messagesStore.deleteMessage(accountId, messageId)
  await refreshMailbox()
  performUndoable(t('shell.messageDeleted'), async () => {
    await messagesStore.restoreMessage(accountId, messageId)
    await refreshMailbox()
  })
}

const archiveCurrentMessage = async () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  const accountId = accountsStore.currentAccountId
  const messageId = messagesStore.selectedMessageId
  await messagesStore.archiveMessage(accountId, messageId)
  await refreshMailbox()
  performUndoable(t('shell.messageArchived'), async () => {
    await messagesStore.restoreMessage(accountId, messageId)
    await refreshMailbox()
  })
}

const restoreCurrentMessage = async () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  if (mailboxesStore.currentFolder?.kind === 'junk') {
    await restoreMessageFromJunk(messagesStore.selectedMessageId)
    return
  }

  await messagesStore.restoreMessage(accountsStore.currentAccountId, messagesStore.selectedMessageId)
  await refreshMailbox()
}

const moveCurrentMessage = async (folderId: string) => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  const accountId = accountsStore.currentAccountId
  const messageId = messagesStore.selectedMessageId
  const originalFolderId = mailboxesStore.currentFolderId
  await messagesStore.moveMessage(accountId, messageId, folderId)
  await refreshMailbox()
  if (originalFolderId) {
    performUndoable(t('shell.messageMoved'), async () => {
      await messagesStore.moveMessage(accountId, messageId, originalFolderId)
      await refreshMailbox()
    })
  }
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
  const accountId = accountsStore.currentAccountId
  const ids = [...messagesStore.selectedIds]
  await messagesStore.batchDelete(accountId)
  await refreshMailbox()
  performUndoable(t('shell.messagesDeleted', { count: ids.length }), async () => {
    for (const id of ids) await messagesStore.restoreMessage(accountId, id)
    await refreshMailbox()
  })
}

const handleBatchArchive = async () => {
  if (!accountsStore.currentAccountId) return
  const accountId = accountsStore.currentAccountId
  const ids = [...messagesStore.selectedIds]
  await messagesStore.batchArchive(accountId)
  await refreshMailbox()
  performUndoable(t('shell.messagesArchived', { count: ids.length }), async () => {
    for (const id of ids) await messagesStore.restoreMessage(accountId, id)
    await refreshMailbox()
  })
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
  const accountId = accountsStore.currentAccountId
  const ids = [...messagesStore.selectedIds]
  const originalFolderId = mailboxesStore.currentFolderId
  await messagesStore.batchMove(accountId, folderId)
  await refreshMailbox()
  if (originalFolderId) {
    performUndoable(t('shell.messagesMoved', { count: ids.length }), async () => {
      for (const id of ids) await messagesStore.moveMessage(accountId, id, originalFolderId)
      await refreshMailbox()
    })
  }
}

// --- Context menu handlers (operate by messageId, not selectedMessage) ---
const findMessage = (messageId: string) =>
  messagesStore.messages.find((m) => m.id === messageId)

const handleContextReply = (messageId: string) => {
  if (!accountsStore.currentAccountId) return
  const msg = findMessage(messageId)
  if (!msg) return
  composerStore.clearFeedback()
  composerStore.openReply(accountsStore.currentAccountId, msg)
}

const handleContextReplyAll = (messageId: string) => {
  if (!accountsStore.currentAccountId || !currentAccount.value) return
  const msg = findMessage(messageId)
  if (!msg) return
  composerStore.clearFeedback()
  composerStore.openReplyAll(accountsStore.currentAccountId, msg, currentAccount.value.email)
}

const handleContextForward = (messageId: string) => {
  if (!accountsStore.currentAccountId) return
  const msg = findMessage(messageId)
  if (!msg) return
  composerStore.clearFeedback()
  composerStore.openForward(accountsStore.currentAccountId, msg)
}

const handleContextToggleRead = async (messageId: string) => {
  if (!accountsStore.currentAccountId) return
  await messagesStore.toggleRead(accountsStore.currentAccountId, messageId)
  await refreshMailbox()
}

const handleContextArchive = async (messageId: string) => {
  if (!accountsStore.currentAccountId) return
  const accountId = accountsStore.currentAccountId
  await messagesStore.archiveMessage(accountId, messageId)
  await refreshMailbox()
  performUndoable(t('shell.messageArchived'), async () => {
    await messagesStore.restoreMessage(accountId, messageId)
    await refreshMailbox()
  })
}

const getFolderIdByKind = (kind: 'inbox' | 'junk') =>
  mailboxesStore.folders.find((folder) => folder.kind === kind)?.id ?? null

const markMessageSpam = async (messageId: string) => {
  if (!accountsStore.currentAccountId) return
  const junkFolderId = getFolderIdByKind('junk')
  const originalFolderId = findMessage(messageId)?.folderId ?? mailboxesStore.currentFolderId
  if (!junkFolderId || !originalFolderId || originalFolderId === junkFolderId) return

  const accountId = accountsStore.currentAccountId
  await messagesStore.moveMessage(accountId, messageId, junkFolderId)
  await refreshMailbox()
  performUndoable(t('shell.messageMoved'), async () => {
    await messagesStore.moveMessage(accountId, messageId, originalFolderId)
    await refreshMailbox()
  })
}

const restoreMessageFromJunk = async (messageId: string) => {
  if (!accountsStore.currentAccountId) return
  const inboxFolderId = getFolderIdByKind('inbox')
  if (!inboxFolderId) return

  await messagesStore.moveMessage(accountsStore.currentAccountId, messageId, inboxFolderId)
  await refreshMailbox()
}

const markCurrentMessageSpam = async () => {
  if (!messagesStore.selectedMessageId) return
  await markMessageSpam(messagesStore.selectedMessageId)
}

const handleContextRestore = async (messageId: string) => {
  if (!accountsStore.currentAccountId) return
  if (mailboxesStore.currentFolder?.kind === 'junk') {
    await restoreMessageFromJunk(messageId)
    return
  }
  await messagesStore.restoreMessage(accountsStore.currentAccountId, messageId)
  await refreshMailbox()
}

const handleContextMarkSpam = async (messageId: string) => {
  await markMessageSpam(messageId)
}

const handleContextDelete = async (messageId: string) => {
  if (!accountsStore.currentAccountId) return
  const accountId = accountsStore.currentAccountId
  await messagesStore.deleteMessage(accountId, messageId)
  await refreshMailbox()
  performUndoable(t('shell.messageDeleted'), async () => {
    await messagesStore.restoreMessage(accountId, messageId)
    await refreshMailbox()
  })
}

const handleContextMove = async (messageId: string, folderId: string) => {
  if (!accountsStore.currentAccountId) return
  const accountId = accountsStore.currentAccountId
  const originalFolderId = mailboxesStore.currentFolderId
  await messagesStore.moveMessage(accountId, messageId, folderId)
  await refreshMailbox()
  if (originalFolderId) {
    performUndoable(t('shell.messageMoved'), async () => {
      await messagesStore.moveMessage(accountId, messageId, originalFolderId)
      await refreshMailbox()
    })
  }
}

// --- Sidebar context menu handlers ---
const handleSaveContact = async (data: { name: string; email: string }) => {
  await contactsStore.createContact({ name: data.name, emails: [data.email] })
}

const handleComposeTo = (data: { name: string; email: string }) => {
  if (!accountsStore.currentAccountId) return
  composerStore.openNew(accountsStore.currentAccountId)
  composerStore.draft.to = data.name ? `${data.name} <${data.email}>` : data.email
}

const handleViewContact = (contact: { id: string }) => {
  router.push('/contacts')
  // The contacts view will load and the user can find the contact
}

const handleSyncAccount = async (accountId: string) => {
  messagesStore.setSyncStatus({
    accountId,
    state: 'syncing',
    message: t('shell.syncInProgress'),
    updatedAt: new Date().toISOString(),
  })

  await messagesStore.syncAccount(accountId)
  if (messagesStore.error) {
    lastFailedAction.value = () => handleSyncAccount(accountId)
  } else {
    await refreshMailbox()
  }
}

const handleMarkFolderRead = async (folderId: string) => {
  if (!accountsStore.currentAccountId) return
  await messagesStore.markAllRead(accountsStore.currentAccountId, folderId)
  await refreshMailbox()
}

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

  await messagesStore.syncAccount(accountsStore.currentAccountId)
  if (messagesStore.error) {
    const accountId = accountsStore.currentAccountId
    lastFailedAction.value = () => handleSyncAccount(accountId)
    return
  }
  await refreshMailbox()
}

onMounted(async () => {
  contactsStore.loadContacts()

  // Skip re-initialization if already loaded (e.g. returning from settings)
  if (accountsStore.accounts.length > 0) return

  await accountsStore.loadAccounts()

  if (accountsStore.currentAccountId) {
    await loadMailbox(accountsStore.currentAccountId)
  }
})

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

  const threads = messagesStore.threadSummaries
  const currentIdx = threads.findIndex((thread) => thread.message.id === messagesStore.selectedMessageId)

  switch (event.key) {
    case 'j':
      if (currentIdx < threads.length - 1) {
        handleSelectMessage(threads[currentIdx + 1].message.id)
      }
      break
    case 'k':
      if (currentIdx > 0) {
        handleSelectMessage(threads[currentIdx - 1].message.id)
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

let backgroundSyncUnsubscribe: (() => void) | undefined

onMounted(() => {
  window.addEventListener('keydown', handleKeyboard)
  backgroundSyncUnsubscribe = window.mailyou?.onBackgroundSync(async (accountId) => {
    if (accountId === accountsStore.currentAccountId) {
      await refreshMailbox()
    }
  })
})

onUnmounted(() => {
  backgroundSyncUnsubscribe?.()
  backgroundSyncUnsubscribe = undefined
  window.removeEventListener('keydown', handleKeyboard)
})

let searchTimer: ReturnType<typeof setTimeout> | null = null

watch(
  () => [messagesStore.query, accountsStore.currentAccountId, mailboxesStore.currentFolderId] as const,
  ([query, accountId, folderId]) => {
    if (searchTimer) {
      clearTimeout(searchTimer)
      searchTimer = null
    }

    if (!accountId) {
      return
    }

    searchTimer = setTimeout(async () => {
      const trimmed = query.trim()
      if (trimmed) {
        await messagesStore.searchMessages(accountId, trimmed)
        return
      }

      if (folderId) {
        await messagesStore.loadMessages(accountId, folderId)
      } else {
        await loadMailbox(accountId)
      }
    }, 200)
  },
)
</script>

<style>
.mail-shell__snackbar .v-snackbar__wrapper {
  max-width: min(480px, calc(100vw - 48px));
}

.mail-shell__snackbar .v-snackbar__content {
  max-height: 8em;
  overflow-y: auto;
  overscroll-behavior: contain;
  user-select: text;
  -webkit-user-select: text;
}

.mail-shell__snackbar--error .v-snackbar__wrapper {
  background:
    linear-gradient(rgba(var(--v-theme-error), 0.1), rgba(var(--v-theme-error), 0.1)),
    rgb(var(--v-theme-background));
  color: rgb(var(--v-theme-error));
}

.mail-shell__snackbar-text {
  word-break: break-word;
}

@media (max-width: 600px) {
  .mail-shell__snackbar .v-snackbar__wrapper {
    max-width: calc(100vw - 32px);
  }
}
</style>
