<template>
  <MailShellLayout
    :search="messagesStore.query"
    :loading-active="loadingBarActive"
    :loading-progress="loadingBarProgress"
    :loading-label="loadingBarLabel"
    @update:search="handleSearchUpdate"
  >
    <template #actions>
      <ToolbarActionButton
        class="mail-shell__sync-button"
        :tooltip="t('shell.sync')"
        prepend-icon="mdi-sync"
        size="default"
        :disabled="!accountsStore.currentAccountId"
        :loading="isSyncing"
        @click="syncCurrentAccount"
      >
        {{ t('shell.sync') }}
      </ToolbarActionButton>
      <ToolbarActionButton
        :tooltip="t('shell.toggleTheme')"
        icon="mdi-theme-light-dark"
        @click="uiStore.toggleAppearance"
      />
      <ToolbarActionButton
        :tooltip="t('contacts.title')"
        icon="mdi-contacts-outline"
        @click="router.push('/contacts')"
      />
      <ToolbarActionButton
        :tooltip="t('shell.settings')"
        icon="mdi-cog-outline"
        @click="router.push('/settings')"
      />
    </template>

    <template #sidebar>
      <MailSidebar
        :accounts="accountsStore.accounts"
        :current-account="accountsStore.currentAccount"
        :current-account-id="accountsStore.currentAccountId"
        :current-folder-id="mailboxesStore.currentFolderId"
        :folders="mailboxesStore.folders"
        :labels="sidebarLabels"
        :current-label="selectedLabel"
        :is-folders-loading="isSyncing"
        @add-account="router.push('/account-setup')"
        @compose="openComposer"
        @delete-account="handleDeleteAccount"
        @edit-account="(id) => router.push('/account-setup/' + id)"
        @select-account="handleAccountChange"
        @select-folder="handleFolderChange"
        @sync-account="handleSyncAccount"
        @mark-folder-read="handleMarkFolderRead"
        @select-label="handleLabelSelect"
        @create-folder="handleCreateFolder"
        @rename-folder="handleRenameFolder"
        @delete-folder="handleDeleteFolder"
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
        :batch-busy="Boolean(messagesStore.batchAction?.active)"
        :selected-ids="messagesStore.selectedIds"
        :title="currentFolderDisplayName"
        :folders="mailboxesStore.folders"
        :current-folder-id="mailboxesStore.currentFolderId"
        :current-folder-kind="mailboxesStore.currentFolder?.kind ?? null"
        @select-message="handleSelectMessage"
        @toggle-star="toggleStar"
        @toggle-selection="({ messageId, shiftKey }) => messagesStore.toggleSelection(messageId, { shiftKey })"
        @select-all="messagesStore.selectAll"
        @clear-selection="messagesStore.clearSelection"
        @mark-all-read="handleMarkAllRead"
        @batch-delete="handleBatchDelete"
        @batch-archive="handleBatchArchive"
        @batch-mark-read="handleBatchMarkRead"
        @batch-mark-unread="handleBatchMarkUnread"
        @batch-move="handleBatchMove"
        @batch-manage-labels="openBatchLabelDialog"
        @context-reply="handleContextReply"
        @context-reply-all="handleContextReplyAll"
        @context-forward="handleContextForward"
        @context-toggle-read="handleContextToggleRead"
        @context-manage-labels="openLabelDialog"
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
        @edit-draft="editCurrentDraft"
        @toggle-read="toggleReadCurrentMessage"
        @manage-labels="openCurrentMessageLabelDialog"
        @move="moveCurrentMessage"
        @save-contact="handleSaveContact"
        @compose-to="handleComposeTo"
        @view-contact="handleViewContact"
        @toggle-star="toggleStarCurrentMessage"
        @export-pdf="exportCurrentMessagePdf"
        @select-thread-message="(messageId) => handleSelectMessage({ messageId, shiftKey: false, accelKey: false })"
      />
    </template>
  </MailShellLayout>

  <ComposerDialog
    :draft="composerStore.draft"
    :draft-status="composerStore.draftStatus"
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
  <AppDialogShell
    v-model="composerStore.showRecoveryDialog"
    :title="t(composerStore.recoveryMode === 'existing' ? 'composer.recovery.existingTitle' : 'composer.recovery.title')"
    :max-width="420"
    persistent
  >
    <v-card-text>{{ t(composerStore.recoveryMode === 'existing' ? 'composer.recovery.existingMessage' : 'composer.recovery.message') }}</v-card-text>
    <template #actions>
      <v-btn variant="text" @click="composerStore.cancelRecovery()">{{ t('common.cancel') }}</v-btn>
      <v-spacer />
      <v-btn variant="text" @click="composerStore.discardAndProceed()">
        {{ t(composerStore.recoveryMode === 'existing' ? 'composer.recovery.useSaved' : 'composer.recovery.discard') }}
      </v-btn>
      <v-btn color="primary" variant="tonal" @click="composerStore.recoverDraft()">
        {{ t(composerStore.recoveryMode === 'existing' ? 'composer.recovery.restoreLocal' : 'composer.recovery.restore') }}
      </v-btn>
    </template>
  </AppDialogShell>

  <!-- Delete confirmation dialog -->
  <AppDialogShell v-model="deleteConfirmDialog" :title="t('shell.confirmDelete')" :max-width="400">
    <v-card-text>{{ t('shell.confirmDeleteText') }}</v-card-text>
    <template #actions>
      <v-spacer />
      <v-btn variant="text" @click="deleteConfirmDialog = false">{{ t('common.cancel') }}</v-btn>
      <v-btn color="error" variant="tonal" @click="confirmDeleteCurrentMessage">{{ t('common.delete') }}</v-btn>
    </template>
  </AppDialogShell>

  <AppDialogShell v-model="labelDialogOpen" :title="t('labels.manageTitle')" :max-width="560">
      <v-card-text>
        <div class="text-body-2 text-medium-emphasis mb-4">
          {{ labelDialogSummary }}
        </div>

        <v-alert v-if="labelDialogError" type="error" variant="tonal" class="mb-4">
          {{ labelDialogError }}
        </v-alert>

        <div v-if="labelDialogLabels.length > 0" class="d-flex flex-wrap ga-2 mb-4">
          <v-chip
            v-for="label in labelDialogLabels"
            :key="label.name"
            :color="isLabelApplied(label.name) ? 'primary' : undefined"
            :variant="isLabelApplied(label.name) ? 'flat' : 'tonal'"
            :disabled="labelDialogBusy"
            @click="toggleMessageLabel(label.name)"
          >
            {{ label.name }} ({{ label.count }})
          </v-chip>
        </div>
        <div v-else class="text-body-2 text-medium-emphasis mb-4">
          {{ t('labels.noLabels') }}
        </div>

        <div class="d-flex ga-2 align-start mb-4">
          <v-text-field
            v-model="labelDraftName"
            :label="labelRenameSource ? t('labels.renameTo') : t('labels.newLabel')"
            density="comfortable"
            hide-details
            variant="outlined"
            class="flex-grow-1"
            @keydown.enter.prevent="submitLabelDraft"
          />
          <v-btn
            color="primary"
            variant="tonal"
            :disabled="!labelDraftName.trim() || labelDialogBusy"
            :loading="labelDialogBusy"
            @click="submitLabelDraft"
          >
            {{ labelRenameSource ? t('common.save') : t('labels.addLabel') }}
          </v-btn>
        </div>

        <div v-if="labelRenameSource" class="d-flex justify-end mb-4">
          <v-btn variant="text" :disabled="labelDialogBusy" @click="cancelLabelRename">
            {{ t('labels.cancelRename') }}
          </v-btn>
        </div>

        <div v-if="labelDialogLabels.length > 0">
          <div class="text-caption text-medium-emphasis mb-2">{{ t('labels.organize') }}</div>
          <v-list density="compact" class="rounded-lg border-thin">
            <v-list-item v-for="label in labelDialogLabels" :key="`manage-${label.name}`">
              <v-list-item-title>{{ label.name }}</v-list-item-title>
              <v-list-item-subtitle>{{ t('labels.usedByCount', { count: label.count }) }}</v-list-item-subtitle>
              <template #append>
                <div class="d-flex ga-1">
                  <v-btn
                    icon="mdi-pencil-outline"
                    size="small"
                    variant="text"
                    :disabled="labelDialogBusy"
                    @click="startRenameLabel(label.name)"
                  />
                  <v-btn
                    icon="mdi-delete-outline"
                    size="small"
                    variant="text"
                    color="error"
                    :disabled="labelDialogBusy"
                    @click="deleteAccountLabel(label.name)"
                  />
                </div>
              </template>
            </v-list-item>
          </v-list>
        </div>
      </v-card-text>
      <template #actions>
        <v-spacer />
        <v-btn variant="text" :disabled="labelDialogBusy" @click="closeLabelDialog">{{ t('common.cancel') }}</v-btn>
      </template>
  </AppDialogShell>

  <v-snackbar
    :model-value="Boolean(composerStore.error)"
    class="mail-shell__snackbar mail-shell__snackbar--error"
    location="bottom right"
    @update:model-value="!$event && composerStore.clearFeedback()"
    @contextmenu="openSnackbarMenu"
  >
    <span class="mail-shell__snackbar-text">{{ composerStore.error }}</span>
    <template #actions>
      <SnackbarActions :actions="composerErrorActions" @select="handleComposerErrorAction" />
    </template>
  </v-snackbar>

  <v-snackbar
    :model-value="Boolean(composerStore.successMessage)"
    class="mail-shell__snackbar mail-shell__snackbar--surface"
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
      <SnackbarActions :actions="messagesErrorActions" @select="handleMessagesErrorAction" />
    </template>
  </v-snackbar>

  <v-snackbar
    :model-value="Boolean(mailboxesStore.error)"
    class="mail-shell__snackbar mail-shell__snackbar--error"
    location="bottom right"
    @update:model-value="!$event && (mailboxesStore.error = null)"
    @contextmenu="openSnackbarMenu"
  >
    <span class="mail-shell__snackbar-text">{{ mailboxesStore.error }}</span>
    <template #actions>
      <SnackbarActions :actions="dismissOnlyActions" @select="handleMailboxErrorAction" />
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
    class="mail-shell__snackbar mail-shell__snackbar--surface"
    :timeout="-1"
    location="bottom right"
  >
    {{ undoableAction?.label }}
    <template #actions>
      <SnackbarActions :actions="undoActions" @select="handleUndoAction" />
    </template>
  </v-snackbar>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref, toRef, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import AppDialogShell from '@/components/ui/AppDialogShell.vue'
import SnackbarActions from '@/components/ui/SnackbarActions.vue'
import ToolbarActionButton from '@/components/ui/ToolbarActionButton.vue'
import MailShellLayout from '@/layouts/MailShellLayout.vue'
import MailList from '@/components/mail/MailList.vue'
import MailReader from '@/components/mail/MailReader.vue'
import MailSidebar from '@/components/mail/MailSidebar.vue'
import ContextMenu from '@/components/ContextMenu.vue'
import { useMailLabelDialog } from '@/composables/useMailLabelDialog'
import { useMailMailboxView } from '@/composables/useMailMailboxView'
import { useMailMessageActions } from '@/composables/useMailMessageActions'
import { useMailSnackbarFeedback } from '@/composables/useMailSnackbarFeedback'
import { useUndoableAction } from '@/composables/useUndoableAction'
import DOMPurify from 'dompurify'
import { useAccountsStore } from '@/stores/accounts'
import { useComposerStore } from '@/stores/composer'
import { useMailboxesStore } from '@/stores/mailboxes'
import { useMessagesStore } from '@/stores/messages'
import { useUiStore } from '@/stores/ui'
import { useContactsStore } from '@/stores/contacts'
import { mailRepository } from '@/services/mail'
import type { MailLabel, MailboxBundle } from '@/types/mail'

const ComposerDialog = defineAsyncComponent(() => import('@/components/mail/ComposerDialog.vue'))

const { t, locale } = useI18n()
const router = useRouter()
const accountsStore = useAccountsStore()
const mailboxesStore = useMailboxesStore()
const messagesStore = useMessagesStore()
const composerStore = useComposerStore()
const uiStore = useUiStore()
const contactsStore = useContactsStore()

const { currentAccount } = storeToRefs(accountsStore)
const { syncStatus } = storeToRefs(messagesStore)

const deleteConfirmDialog = ref(false)
const selectedLabel = ref<string | null>(null)
const lastFailedAction = ref<(() => Promise<void>) | null>(null)

const currentFolderDisplayName = computed(() => {
  if (messagesStore.hasSearchQuery) {
    return t('mailList.searchResultsTitle')
  }
  if (selectedLabel.value) {
    return selectedLabel.value
  }
  const folder = mailboxesStore.currentFolder
  if (!folder) return t('common.mailbox')
  return folder.kind !== 'custom' ? t(`folders.${folder.kind}`) : folder.name
})

const isSyncing = computed(() => syncStatus.value?.state === 'syncing')

const {
  applyMailboxView,
  clearMailboxCaches,
  currentMailboxBundle,
  fetchAccountLabels,
  loadMailbox,
  loadingBarActive,
  loadingBarLabel,
  loadingBarProgress,
  prewarmMailboxCaches,
  refreshMailbox: refreshMailboxState,
  setLoadingStage,
  sidebarLabels,
} = useMailMailboxView({
  t,
  isSyncing,
  selectedLabel,
  messagesStore,
  mailboxesStore,
})

const refreshMailbox = (options?: { reloadLabels?: boolean }) =>
  refreshMailboxState(accountsStore.currentAccountId, options)

const patchCachedMessage = (messageId: string, updater: (message: import('@/types/mail').MailMessage) => import('@/types/mail').MailMessage) => {
  if (!currentMailboxBundle.value) {
    return
  }

  currentMailboxBundle.value = {
    ...currentMailboxBundle.value,
    messages: currentMailboxBundle.value.messages.map((message) =>
      message.id === messageId ? updater(message) : message,
    ),
  }
}

const patchCachedMessages = (messageIds: Iterable<string>, updater: (message: import('@/types/mail').MailMessage) => import('@/types/mail').MailMessage) => {
  if (!currentMailboxBundle.value) {
    return
  }

  const ids = new Set(messageIds)
  if (ids.size === 0) {
    return
  }

  currentMailboxBundle.value = {
    ...currentMailboxBundle.value,
    messages: currentMailboxBundle.value.messages.map((message) =>
      ids.has(message.id) ? updater(message) : message,
    ),
  }
}

const applyCachedFolderMove = (messageIds: Iterable<string>, folderId: string) => {
  patchCachedMessages(messageIds, (message) => ({ ...message, folderId }))
}

const applyCachedReadState = (messageIds: Iterable<string>, isRead: boolean) => {
  patchCachedMessages(messageIds, (message) => ({ ...message, isRead }))
}

const adjustCachedFolderCounts = (changes: Array<{ folderId: string; unreadDelta?: number; totalDelta?: number }>) => {
  if (!currentMailboxBundle.value || changes.length === 0) {
    return
  }

  const deltas = new Map<string, { unreadDelta: number; totalDelta: number }>()
  for (const change of changes) {
    const current = deltas.get(change.folderId) ?? { unreadDelta: 0, totalDelta: 0 }
    current.unreadDelta += change.unreadDelta ?? 0
    current.totalDelta += change.totalDelta ?? 0
    deltas.set(change.folderId, current)
  }

  currentMailboxBundle.value = {
    ...currentMailboxBundle.value,
    folders: currentMailboxBundle.value.folders.map((folder) => {
      const delta = deltas.get(folder.id)
      if (!delta) {
        return folder
      }

      return {
        ...folder,
        unreadCount: Math.max(0, folder.unreadCount + delta.unreadDelta),
        totalCount: Math.max(0, folder.totalCount + delta.totalDelta),
      }
    }),
  }
}

const adjustUnreadCountsForMessages = (
  messages: Array<Pick<import('@/types/mail').MailMessage, 'folderId' | 'isRead'>>,
  nextIsRead: boolean,
) => {
  const perFolderDelta = new Map<string, number>()

  for (const message of messages) {
    if (message.isRead === nextIsRead) {
      continue
    }

    const delta = nextIsRead ? -1 : 1
    perFolderDelta.set(message.folderId, (perFolderDelta.get(message.folderId) ?? 0) + delta)
  }

  for (const [folderId, delta] of perFolderDelta) {
    mailboxesStore.adjustUnread(folderId, delta)
  }
}

const handleSearchUpdate = (value: string) => {
  messagesStore.query = value
  if (value.trim()) {
    selectedLabel.value = null
  }
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

const {
  dismissUndo,
  handleUndoAction,
  performUndoable,
  undoActions,
  undoableAction,
} = useUndoableAction(t)

const {
  composerErrorActions,
  dismissOnlyActions,
  handleComposerErrorAction,
  handleMailboxErrorAction,
  handleMessagesErrorAction,
  messagesErrorActions,
  openSnackbarMenu,
  snackbarCopy,
  snackbarCtx,
  snackbarHasSelection,
  snackbarSelectAll,
} = useMailSnackbarFeedback({
  t,
  retrySend: () => retrySend(),
  retryLastAction: () => retryLastAction(),
  clearComposerFeedback: () => composerStore.clearFeedback(),
  clearMessagesError: () => messagesStore.clearError(),
  clearMailboxError: () => {
    mailboxesStore.error = null
  },
  getFallbackCopyText: () => messagesStore.error || composerStore.error || '',
})

const handleSelectMessage = async ({
  messageId,
  shiftKey,
  accelKey,
}: {
  messageId: string
  shiftKey: boolean
  accelKey: boolean
}) => {
  if (shiftKey) {
    messagesStore.selectMessageRange(messageId)
  } else if (accelKey) {
    messagesStore.toggleSelection(messageId)
    messagesStore.selectMessage(messageId)
  } else {
    messagesStore.selectMessage(messageId)
  }

  if (!accountsStore.currentAccountId) {
    return
  }

  if (shiftKey || accelKey) {
    return
  }

  const message = messagesStore.messages.find((m) => m.id === messageId)
  if (message && !message.isRead) {
    mailboxesStore.decrementUnread(message.folderId)
    patchCachedMessage(messageId, (cached) => ({ ...cached, isRead: true }))
    try {
      await messagesStore.toggleRead(accountsStore.currentAccountId, messageId)
    } catch {
      mailboxesStore.adjustUnread(message.folderId, 1)
      patchCachedMessage(messageId, (cached) => ({ ...cached, isRead: false }))
    }
  }
}

const handleAccountChange = async (accountId: string) => {
  accountsStore.selectAccount(accountId)
  messagesStore.clearSelection()
  messagesStore.clearError()
  selectedLabel.value = null
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
  currentMailboxBundle.value = null
  sidebarLabels.value = []
  clearMailboxCaches()
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
  selectedLabel.value = null
  mailboxesStore.selectFolder(folderId)
  try {
    const canReuseMailboxBundle = currentMailboxBundle.value?.folders.some((folder) => folder.id === folderId)

    if (canReuseMailboxBundle && currentMailboxBundle.value) {
      setLoadingStage('applying')
      messagesStore.setMailboxBundle(currentMailboxBundle.value, folderId)
      setLoadingStage('finalizing')
      return
    }

    await loadMailbox(accountsStore.currentAccountId)
  } catch {
    messagesStore.error = t('shell.failedToLoadMessages')
  } finally {
    setLoadingStage('idle')
  }
}

const handleLabelSelect = async (label: string) => {
  if (!accountsStore.currentAccountId) return
  messagesStore.clearSelection()
  messagesStore.query = ''
  selectedLabel.value = label
  if (currentMailboxBundle.value) {
    await applyMailboxView(accountsStore.currentAccountId, currentMailboxBundle.value)
    return
  }
  await loadMailbox(accountsStore.currentAccountId)
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

const editCurrentDraft = async () => {
  const message = messagesStore.selectedMessage
  if (!message || !accountsStore.currentAccountId) {
    return
  }

  const draft = await mailRepository.getDraft(accountsStore.currentAccountId, message.id)
  if (!draft) {
    messagesStore.error = 'Draft no longer exists'
    return
  }

  composerStore.openExistingDraft(draft)
}

const retrySend = async () => {
  composerStore.clearFeedback()
  await sendDraft()
}

const toggleStar = async (messageId: string) => {
  if (!accountsStore.currentAccountId) {
    return
  }

  const before = messagesStore.messages.find((message) => message.id === messageId)
  if (before) {
    patchCachedMessage(messageId, (message) => ({ ...message, isStarred: !before.isStarred }))
  }

  try {
    await messagesStore.toggleStar(accountsStore.currentAccountId, messageId)
    const updated = messagesStore.messages.find((message) => message.id === messageId)
    if (updated) {
      patchCachedMessage(messageId, () => updated)
    }
  } catch {
    if (before) {
      patchCachedMessage(messageId, (message) => ({ ...message, isStarred: before.isStarred }))
    }
  }
}

const toggleReadCurrentMessage = async () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  const before = messagesStore.messages.find((message) => message.id === messagesStore.selectedMessageId)
  let optimisticUnreadDelta = 0
  if (before) {
    const optimisticRead = !before.isRead
    optimisticUnreadDelta = optimisticRead === before.isRead ? 0 : (optimisticRead ? -1 : 1)
    mailboxesStore.adjustUnread(before.folderId, optimisticUnreadDelta)
    patchCachedMessage(before.id, (message) => ({ ...message, isRead: optimisticRead }))
  }
  try {
    await messagesStore.toggleRead(accountsStore.currentAccountId, messagesStore.selectedMessageId)
    const after = messagesStore.messages.find((message) => message.id === messagesStore.selectedMessageId)
    if (after) {
      patchCachedMessage(after.id, () => after)
    }
  } catch {
    if (before) {
      mailboxesStore.adjustUnread(before.folderId, -optimisticUnreadDelta)
      patchCachedMessage(before.id, (message) => ({ ...message, isRead: before.isRead }))
    }
  }
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

// --- Context menu handlers (operate by messageId, not selectedMessage) ---
const {
  archiveCurrentMessage,
  confirmDeleteCurrentMessage: runDeleteCurrentMessage,
  findMessage,
  handleBatchArchive,
  handleBatchDelete,
  handleBatchMarkRead,
  handleBatchMarkUnread,
  handleBatchMove,
  handleContextArchive,
  handleContextDelete,
  handleContextMarkSpam,
  handleContextMove,
  handleContextRestore,
  handleContextToggleRead,
  handleCreateFolder,
  handleDeleteFolder,
  handleMarkAllRead,
  handleRenameFolder,
  markCurrentMessageSpam,
  moveCurrentMessage,
  restoreCurrentMessage,
} = useMailMessageActions({
  t,
  currentAccountId: toRef(accountsStore, 'currentAccountId'),
  messagesStore,
  mailboxesStore,
  refreshMailbox,
  performUndoable,
  applyCachedReadState,
  applyCachedFolderMove,
  adjustCachedFolderCounts,
  adjustUnreadCountsForMessages,
  patchCachedMessage,
})

const confirmDeleteCurrentMessage = async () => {
  deleteConfirmDialog.value = false
  await runDeleteCurrentMessage()
}

const {
  cancelLabelRename,
  closeLabelDialog,
  deleteAccountLabel,
  isLabelApplied,
  labelDialogBusy,
  labelDialogError,
  labelDialogLabels,
  labelDialogOpen,
  labelDialogSummary,
  labelDraftName,
  labelRenameSource,
  openLabelDialog,
  openLabelDialogForMessages,
  startRenameLabel,
  submitLabelDraft,
  toggleMessageLabel,
} = useMailLabelDialog({
  t,
  currentAccountId: () => accountsStore.currentAccountId,
  findMessage,
  fetchAccountLabels,
  refreshMailbox,
})

const openCurrentMessageLabelDialog = async () => {
  if (!messagesStore.selectedMessageId) return
  await openLabelDialog(messagesStore.selectedMessageId)
}

const openBatchLabelDialog = async () => {
  if (messagesStore.selectedIds.size === 0) return
  await openLabelDialogForMessages([...messagesStore.selectedIds])
}

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
  setLoadingStage('syncing')
  try {
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
      await refreshMailbox({ reloadLabels: true })
    }
  } finally {
    setLoadingStage('idle')
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

  setLoadingStage('syncing')
  try {
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
    await refreshMailbox({ reloadLabels: true })
  } finally {
    setLoadingStage('idle')
  }
}

onMounted(async () => {
  contactsStore.loadContacts()

  // Skip re-initialization if already loaded (e.g. returning from settings)
  if (accountsStore.accounts.length > 0) return

  await accountsStore.loadAccounts()

  if (accountsStore.currentAccountId) {
    await loadMailbox(accountsStore.currentAccountId)
  }

  void prewarmMailboxCaches(
    accountsStore.accounts.map((account) => account.id),
    accountsStore.currentAccountId,
  )
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
  const isAccel = event.ctrlKey || event.metaKey

  if (isAccel && event.key.toLowerCase() === 'a') {
    event.preventDefault()
    messagesStore.selectAll()
    return
  }

  switch (event.key) {
    case 'j':
      if (currentIdx < threads.length - 1) {
        handleSelectMessage({ messageId: threads[currentIdx + 1].message.id, shiftKey: event.shiftKey, accelKey: false })
      }
      break
    case 'k':
      if (currentIdx > 0) {
        handleSelectMessage({ messageId: threads[currentIdx - 1].message.id, shiftKey: event.shiftKey, accelKey: false })
      }
      break
    case 'ArrowDown':
      if (currentIdx < threads.length - 1) {
        event.preventDefault()
        handleSelectMessage({ messageId: threads[currentIdx + 1].message.id, shiftKey: event.shiftKey, accelKey: false })
      }
      break
    case 'ArrowUp':
      if (currentIdx > 0) {
        event.preventDefault()
        handleSelectMessage({ messageId: threads[currentIdx - 1].message.id, shiftKey: event.shiftKey, accelKey: false })
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
  () => accountsStore.accounts.map((account) => account.id).join(','),
  () => {
    void prewarmMailboxCaches(
      accountsStore.accounts.map((account) => account.id),
      accountsStore.currentAccountId,
    )
  },
)

watch(
  () => [messagesStore.query, accountsStore.currentAccountId, mailboxesStore.currentFolderId, selectedLabel.value] as const,
  ([query, accountId, folderId, label]) => {
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
        setLoadingStage('searching')
        await messagesStore.searchMessages(accountId, trimmed)
        setLoadingStage('idle')
        return
      }

      if (label) {
        if (currentMailboxBundle.value) {
          await applyMailboxView(accountId, currentMailboxBundle.value)
        } else {
          await loadMailbox(accountId)
        }
        return
      }

      if (folderId) {
        const canReuseMailboxBundle = currentMailboxBundle.value?.folders.some((folder) => folder.id === folderId)

        if (canReuseMailboxBundle && currentMailboxBundle.value) {
          setLoadingStage('applying')
          messagesStore.setMailboxBundle(currentMailboxBundle.value, folderId)
          setLoadingStage('finalizing')
        } else {
          setLoadingStage('fetching')
          await messagesStore.loadMessages(accountId, folderId)
        }
        setLoadingStage('idle')
      } else {
        await loadMailbox(accountId)
      }
    }, 200)
  },
)
</script>

<style>
.mail-shell__sync-button {
  min-width: 0;
}

.mail-shell__sync-button.v-btn {
  min-height: 38px;
  padding-inline: 15px 17px;
  font-weight: 600;
  letter-spacing: 0.01em;
  background: rgba(var(--v-theme-surface-variant), 0.72);
  border: 1px solid rgba(var(--v-theme-outline), 0.16);
  box-shadow: 0 1px 0 rgba(var(--v-theme-on-surface), 0.04);
}

.mail-shell__sync-button:deep(.v-btn__content) {
  gap: 9px;
}

.mail-shell__snackbar .v-snackbar__wrapper {
  max-width: min(480px, calc(100vw - 48px));
  color: rgb(var(--v-theme-on-surface));
}

.mail-shell__snackbar .v-btn,
.mail-shell__snackbar .v-icon {
  color: inherit;
}

.mail-shell__snackbar .v-snackbar__content {
  max-height: 8em;
  overflow-y: auto;
  overscroll-behavior: contain;
  user-select: text;
  -webkit-user-select: text;
}

.mail-shell__snackbar--surface .v-snackbar__wrapper {
  background:
    linear-gradient(rgba(var(--v-theme-surface-variant), 0.82), rgba(var(--v-theme-surface-variant), 0.82)),
    rgb(var(--v-theme-surface));
  color: rgb(var(--v-theme-on-surface));
}

.mail-shell__snackbar--error .v-snackbar__wrapper {
  background:
    linear-gradient(rgba(var(--v-theme-error), 0.12), rgba(var(--v-theme-error), 0.12)),
    rgb(var(--v-theme-surface));
  color: rgb(var(--v-theme-on-surface));
  box-shadow: inset 3px 0 0 rgb(var(--v-theme-error));
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
