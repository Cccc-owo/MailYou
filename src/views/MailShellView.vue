<template>
  <MailShellLayout
    :search="messagesStore.query"
    :loading-active="loadingBarActive"
    :loading-progress="loadingBarProgress"
    :loading-label="loadingBarLabel"
    @update:search="handleSearchUpdate"
  >
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
        @select-thread-message="handleSelectMessage"
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
  <v-dialog v-model="composerStore.showRecoveryDialog" max-width="420" persistent>
    <v-card>
      <v-card-title>{{ t(composerStore.recoveryMode === 'existing' ? 'composer.recovery.existingTitle' : 'composer.recovery.title') }}</v-card-title>
      <v-card-text>{{ t(composerStore.recoveryMode === 'existing' ? 'composer.recovery.existingMessage' : 'composer.recovery.message') }}</v-card-text>
      <v-card-actions>
        <v-btn variant="text" @click="composerStore.cancelRecovery()">{{ t('common.cancel') }}</v-btn>
        <v-spacer />
        <v-btn variant="text" @click="composerStore.discardAndProceed()">
          {{ t(composerStore.recoveryMode === 'existing' ? 'composer.recovery.useSaved' : 'composer.recovery.discard') }}
        </v-btn>
        <v-btn color="primary" variant="tonal" @click="composerStore.recoverDraft()">
          {{ t(composerStore.recoveryMode === 'existing' ? 'composer.recovery.restoreLocal' : 'composer.recovery.restore') }}
        </v-btn>
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

  <v-dialog v-model="labelDialogOpen" max-width="560">
    <v-card>
      <v-card-title>{{ t('labels.manageTitle') }}</v-card-title>
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
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" :disabled="labelDialogBusy" @click="closeLabelDialog">{{ t('common.cancel') }}</v-btn>
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

  <v-snackbar
    :model-value="Boolean(mailboxesStore.error)"
    class="mail-shell__snackbar mail-shell__snackbar--error"
    location="bottom right"
    @update:model-value="!$event && (mailboxesStore.error = null)"
    @contextmenu="openSnackbarMenu"
  >
    <span class="mail-shell__snackbar-text">{{ mailboxesStore.error }}</span>
    <template #actions>
      <v-btn variant="text" @click="mailboxesStore.error = null">{{ t('common.dismiss') }}</v-btn>
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
import type { MailLabel, MailboxBundle } from '@/types/mail'

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
const selectedLabel = ref<string | null>(null)
const labelDialogOpen = ref(false)
const labelDialogMessageIds = ref<string[]>([])
const labelDialogBusy = ref(false)
const labelDialogError = ref<string | null>(null)
const labelDraftName = ref('')
const labelRenameSource = ref<string | null>(null)
const sidebarLabels = ref<MailLabel[]>([])
const labelDialogLabels = ref<MailLabel[]>([])
const currentMailboxBundle = ref<MailboxBundle | null>(null)
const lastFailedAction = ref<(() => Promise<void>) | null>(null)
const mailboxRequestGeneration = ref(0)
let mailboxLoadPromise: Promise<MailboxBundle> | null = null
let mailboxLoadAccountId: string | null = null
let labelsLoadPromise: Promise<MailLabel[]> | null = null
let labelsLoadAccountId: string | null = null
let lastMailboxLoadedAt = 0
let lastLabelsLoadedAt = 0
let refreshMailboxPromise: Promise<void> | null = null
let refreshMailboxPending = false

const MAILBOX_CACHE_WINDOW_MS = 1200
const LABEL_CACHE_WINDOW_MS = 1500
type LoadingStage = 'idle' | 'syncing' | 'fetching' | 'applying' | 'searching' | 'finalizing'
const loadingStage = ref<LoadingStage>('idle')

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
  if (selectedLabel.value) {
    return selectedLabel.value
  }
  const folder = mailboxesStore.currentFolder
  if (!folder) return t('common.mailbox')
  return folder.kind !== 'custom' ? t(`folders.${folder.kind}`) : folder.name
})

const isSyncing = computed(() => syncStatus.value?.state === 'syncing')
const loadingBarActive = computed(() =>
  isSyncing.value || messagesStore.isLoading || loadingStage.value !== 'idle',
)
const loadingBarProgress = computed(() => {
  switch (loadingStage.value) {
    case 'syncing':
      return 18
    case 'fetching':
      return 42
    case 'applying':
      return 72
    case 'searching':
      return 84
    case 'finalizing':
      return 96
    default:
      return isSyncing.value || messagesStore.isLoading ? null : 100
  }
})
const loadingBarLabel = computed(() => {
  switch (loadingStage.value) {
    case 'syncing':
      return t('shell.syncInProgress')
    case 'fetching':
      return t('shell.loadingMail')
    case 'applying':
      return t('shell.applyingMailboxChanges')
    case 'searching':
      return t('shell.searchingMail')
    case 'finalizing':
      return t('shell.finalizingMailbox')
    default:
      return ''
  }
})

const setLoadingStage = (stage: LoadingStage) => {
  loadingStage.value = stage
}

const buildLabelFilteredMessages = (bundle: MailboxBundle, label: string) =>
  bundle.messages
    .filter((message) => message.labels.some((item) => item.toLowerCase() === label.toLowerCase()))
    .sort((left, right) => new Date(right.receivedAt).getTime() - new Date(left.receivedAt).getTime())

const fetchMailboxBundle = async (accountId: string, options?: { force?: boolean }) => {
  const force = options?.force ?? false
  const now = Date.now()

  if (!force
    && currentMailboxBundle.value
    && mailboxLoadAccountId === accountId
    && now - lastMailboxLoadedAt < MAILBOX_CACHE_WINDOW_MS) {
    return currentMailboxBundle.value
  }

  if (!force && mailboxLoadPromise && mailboxLoadAccountId === accountId) {
    return mailboxLoadPromise
  }

  mailboxLoadAccountId = accountId
  mailboxLoadPromise = mailRepository.getMailboxBundle(accountId)
    .then((bundle) => {
      currentMailboxBundle.value = bundle
      lastMailboxLoadedAt = Date.now()
      return bundle
    })
    .finally(() => {
      mailboxLoadPromise = null
    })

  return mailboxLoadPromise
}

const fetchAccountLabels = async (accountId: string, options?: { force?: boolean }) => {
  const force = options?.force ?? false
  const now = Date.now()

  if (!force
    && sidebarLabels.value.length > 0
    && labelsLoadAccountId === accountId
    && now - lastLabelsLoadedAt < LABEL_CACHE_WINDOW_MS) {
    return sidebarLabels.value
  }

  if (!force && labelsLoadPromise && labelsLoadAccountId === accountId) {
    return labelsLoadPromise
  }

  labelsLoadAccountId = accountId
  labelsLoadPromise = mailRepository.listLabels(accountId)
    .then((labels) => {
      sidebarLabels.value = labels
      lastLabelsLoadedAt = Date.now()
      return labels
    })
    .finally(() => {
      labelsLoadPromise = null
    })

  return labelsLoadPromise
}

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

const applyCachedReadState = (messageIds: Iterable<string>, isRead: boolean) => {
  patchCachedMessages(messageIds, (message) => ({ ...message, isRead }))
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

const applyMailboxView = async (accountId: string, bundle?: MailboxBundle) => {
  const requestId = ++mailboxRequestGeneration.value
  const mailboxBundle = bundle ?? await fetchMailboxBundle(accountId)
  setLoadingStage('applying')
  const labels = await fetchAccountLabels(accountId)
  if (requestId !== mailboxRequestGeneration.value) {
    return
  }

  currentMailboxBundle.value = mailboxBundle
  mailboxesStore.setFolders(mailboxBundle.folders)
  sidebarLabels.value = labels

  if (messagesStore.hasSearchQuery) {
    setLoadingStage('searching')
    await messagesStore.searchMessages(accountId, messagesStore.query)
    setLoadingStage('finalizing')
    return
  }

  if (selectedLabel.value) {
    messagesStore.setSyncStatus(mailboxBundle.syncStatus)
    messagesStore.setMessages(buildLabelFilteredMessages(mailboxBundle, selectedLabel.value))
    return
  }

  messagesStore.setMailboxBundle(mailboxBundle, mailboxesStore.currentFolderId)
}

const loadMailbox = async (accountId: string) => {
  messagesStore.isLoading = true

  try {
    setLoadingStage('fetching')
    const bundle = await fetchMailboxBundle(accountId, { force: true })
    await applyMailboxView(accountId, bundle)
    setLoadingStage('finalizing')
  } finally {
    messagesStore.isLoading = false
    setLoadingStage('idle')
  }
}

const refreshMailbox = async () => {
  if (!accountsStore.currentAccountId) {
    return
  }

  if (refreshMailboxPromise) {
    refreshMailboxPending = true
    return refreshMailboxPromise
  }

  refreshMailboxPromise = (async () => {
    do {
      const accountId = accountsStore.currentAccountId
      if (!accountId) {
        refreshMailboxPending = false
        break
      }
      refreshMailboxPending = false
      mailboxLoadPromise = null
      labelsLoadPromise = null
      lastMailboxLoadedAt = 0
      lastLabelsLoadedAt = 0
      await loadMailbox(accountId)
    } while (refreshMailboxPending)
  })().finally(() => {
    refreshMailboxPromise = null
  })

  return refreshMailboxPromise
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

const handleSelectMessage = async (messageId: string) => {
  messagesStore.selectMessage(messageId)

  if (!accountsStore.currentAccountId) {
    return
  }

  const message = messagesStore.messages.find((m) => m.id === messageId)
  if (message && !message.isRead) {
    await messagesStore.toggleRead(accountsStore.currentAccountId, messageId)
    mailboxesStore.decrementUnread(message.folderId)
    patchCachedMessage(messageId, (cached) => ({ ...cached, isRead: true }))
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
  mailboxLoadPromise = null
  mailboxLoadAccountId = null
  labelsLoadPromise = null
  labelsLoadAccountId = null
  lastMailboxLoadedAt = 0
  lastLabelsLoadedAt = 0
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
    setLoadingStage('fetching')
    await messagesStore.loadMessages(accountsStore.currentAccountId, folderId)
    setLoadingStage('finalizing')
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

  await messagesStore.toggleStar(accountsStore.currentAccountId, messageId)
  const updated = messagesStore.messages.find((message) => message.id === messageId)
  if (updated) {
    patchCachedMessage(messageId, () => updated)
  }
}

const toggleReadCurrentMessage = async () => {
  if (!accountsStore.currentAccountId || !messagesStore.selectedMessageId) {
    return
  }

  const before = messagesStore.messages.find((message) => message.id === messagesStore.selectedMessageId)
  await messagesStore.toggleRead(accountsStore.currentAccountId, messagesStore.selectedMessageId)
  const after = messagesStore.messages.find((message) => message.id === messagesStore.selectedMessageId)
  if (before && after) {
    mailboxesStore.adjustUnread(before.folderId, after.isRead === before.isRead ? 0 : (after.isRead ? -1 : 1))
    patchCachedMessage(after.id, () => after)
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

  const unreadMessages = messagesStore.messages
    .filter((message) => !message.isRead)
    .map((message) => ({ folderId: message.folderId, isRead: message.isRead }))
  await messagesStore.markAllRead(accountsStore.currentAccountId, mailboxesStore.currentFolderId)
  adjustUnreadCountsForMessages(unreadMessages, true)
  applyCachedReadState(messagesStore.messages.map((message) => message.id), true)
}

const handleCreateFolder = async (name: string) => {
  if (!accountsStore.currentAccountId) {
    return
  }

  try {
    await mailboxesStore.createFolder(accountsStore.currentAccountId, name)
    await refreshMailbox()
  } catch (error) {
    mailboxesStore.error = error instanceof Error ? error.message : 'Unable to create folder'
  }
}

const handleRenameFolder = async (folderId: string, name: string) => {
  if (!accountsStore.currentAccountId) {
    return
  }

  try {
    await mailboxesStore.renameFolder(accountsStore.currentAccountId, folderId, name)
    await refreshMailbox()
  } catch (error) {
    mailboxesStore.error = error instanceof Error ? error.message : 'Unable to rename folder'
  }
}

const handleDeleteFolder = async (folderId: string) => {
  if (!accountsStore.currentAccountId) {
    return
  }

  try {
    await mailboxesStore.deleteFolder(accountsStore.currentAccountId, folderId)
    await refreshMailbox()
  } catch (error) {
    mailboxesStore.error = error instanceof Error ? error.message : 'Unable to delete folder'
  }
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
  const selectedIds = [...messagesStore.selectedIds]
  const affectedMessages = selectedIds
    .map((id) => messagesStore.messages.find((message) => message.id === id))
    .filter((message): message is NonNullable<typeof message> => Boolean(message))
  await messagesStore.batchToggleRead(accountsStore.currentAccountId, true)
  adjustUnreadCountsForMessages(affectedMessages, true)
  applyCachedReadState(selectedIds, true)
}

const handleBatchMarkUnread = async () => {
  if (!accountsStore.currentAccountId) return
  const selectedIds = [...messagesStore.selectedIds]
  const affectedMessages = selectedIds
    .map((id) => messagesStore.messages.find((message) => message.id === id))
    .filter((message): message is NonNullable<typeof message> => Boolean(message))
  await messagesStore.batchToggleRead(accountsStore.currentAccountId, false)
  adjustUnreadCountsForMessages(affectedMessages, false)
  applyCachedReadState(selectedIds, false)
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

const labelDialogMessages = computed(() =>
  labelDialogMessageIds.value
    .map((messageId) => findMessage(messageId))
    .filter((message): message is NonNullable<typeof message> => Boolean(message)),
)

const labelDialogSummary = computed(() => {
  if (labelDialogMessages.value.length === 1) {
    return labelDialogMessages.value[0].subject || t('labels.manageHint')
  }
  if (labelDialogMessages.value.length > 1) {
    return t('labels.selectedMessages', { count: labelDialogMessages.value.length })
  }
  return t('labels.manageHint')
})

const resetLabelDialogState = () => {
  labelDialogBusy.value = false
  labelDialogError.value = null
  labelDraftName.value = ''
  labelRenameSource.value = null
  labelDialogLabels.value = []
}

const loadAccountLabels = async () => {
  if (!accountsStore.currentAccountId) {
    labelDialogLabels.value = []
    return
  }

  labelDialogLabels.value = await fetchAccountLabels(accountsStore.currentAccountId, { force: true })
}

const openLabelDialog = async (messageId: string) => {
  await openLabelDialogForMessages([messageId])
}

const openLabelDialogForMessages = async (messageIds: string[]) => {
  if (!accountsStore.currentAccountId) return
  labelDialogMessageIds.value = messageIds
  resetLabelDialogState()
  labelDialogOpen.value = true
  try {
    await loadAccountLabels()
  } catch (error) {
    labelDialogError.value = error instanceof Error ? error.message : t('labels.loadFailed')
  }
}

const openCurrentMessageLabelDialog = async () => {
  if (!messagesStore.selectedMessageId) return
  await openLabelDialog(messagesStore.selectedMessageId)
}

const openBatchLabelDialog = async () => {
  if (messagesStore.selectedIds.size === 0) return
  await openLabelDialogForMessages([...messagesStore.selectedIds])
}

const closeLabelDialog = () => {
  labelDialogOpen.value = false
  labelDialogMessageIds.value = []
  resetLabelDialogState()
}

const isLabelApplied = (label: string) =>
  labelDialogMessages.value.length > 0
    && labelDialogMessages.value.every((message) =>
      message.labels.some((item) => item.toLowerCase() === label.toLowerCase()),
    )

const toggleMessageLabel = async (label: string) => {
  if (!accountsStore.currentAccountId || labelDialogMessageIds.value.length === 0) return

  labelDialogBusy.value = true
  labelDialogError.value = null
  try {
    const applyToAll = !isLabelApplied(label)
    for (const messageId of labelDialogMessageIds.value) {
      if (applyToAll) {
        await mailRepository.addLabel(accountsStore.currentAccountId, messageId, label)
      } else {
        await mailRepository.removeLabel(accountsStore.currentAccountId, messageId, label)
      }
    }
    await refreshMailbox()
    await loadAccountLabels()
  } catch (error) {
    labelDialogError.value = error instanceof Error ? error.message : t('labels.updateFailed')
  } finally {
    labelDialogBusy.value = false
  }
}

const startRenameLabel = (label: string) => {
  labelRenameSource.value = label
  labelDraftName.value = label
  labelDialogError.value = null
}

const cancelLabelRename = () => {
  labelRenameSource.value = null
  labelDraftName.value = ''
  labelDialogError.value = null
}

const submitLabelDraft = async () => {
  if (!accountsStore.currentAccountId || labelDialogMessageIds.value.length === 0 || !labelDraftName.value.trim()) return

  labelDialogBusy.value = true
  labelDialogError.value = null
  try {
    if (labelRenameSource.value) {
      labelDialogLabels.value = await mailRepository.renameLabel(
        accountsStore.currentAccountId,
        labelRenameSource.value,
        labelDraftName.value,
      )
      await refreshMailbox()
      cancelLabelRename()
      return
    }

    for (const messageId of labelDialogMessageIds.value) {
      await mailRepository.addLabel(accountsStore.currentAccountId, messageId, labelDraftName.value)
    }
    labelDraftName.value = ''
    await refreshMailbox()
    await loadAccountLabels()
  } catch (error) {
    labelDialogError.value = error instanceof Error ? error.message : t('labels.updateFailed')
  } finally {
    labelDialogBusy.value = false
  }
}

const deleteAccountLabel = async (label: string) => {
  if (!accountsStore.currentAccountId) return
  if (!window.confirm(t('labels.deleteConfirm', { label }))) return

  labelDialogBusy.value = true
  labelDialogError.value = null
  try {
    labelDialogLabels.value = await mailRepository.deleteLabel(accountsStore.currentAccountId, label)
    await refreshMailbox()
    if (labelRenameSource.value?.toLowerCase() === label.toLowerCase()) {
      cancelLabelRename()
    }
  } catch (error) {
    labelDialogError.value = error instanceof Error ? error.message : t('labels.updateFailed')
  } finally {
    labelDialogBusy.value = false
  }
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

const handleContextToggleRead = async (messageId: string) => {
  if (!accountsStore.currentAccountId) return
  const before = messagesStore.messages.find((message) => message.id === messageId)
  await messagesStore.toggleRead(accountsStore.currentAccountId, messageId)
  const after = messagesStore.messages.find((message) => message.id === messageId)
  if (before && after) {
    mailboxesStore.adjustUnread(before.folderId, after.isRead === before.isRead ? 0 : (after.isRead ? -1 : 1))
    patchCachedMessage(messageId, () => after)
  }
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
      await refreshMailbox()
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
    await refreshMailbox()
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
        setLoadingStage('fetching')
        await messagesStore.loadMessages(accountId, folderId)
        setLoadingStage('idle')
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
