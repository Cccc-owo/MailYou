import { computed, ref, type Ref } from 'vue'
import { mailRepository } from '@/services/mail'
import type { MailLabel, MailMessage, MailboxBundle } from '@/types/mail'

type LoadingStage = 'idle' | 'syncing' | 'fetching' | 'applying' | 'searching' | 'finalizing'

interface UseMailMailboxViewOptions {
  t: (key: string, params?: Record<string, unknown>) => string
  isSyncing: Ref<boolean>
  selectedLabel: Ref<string | null>
  messagesStore: {
    batchAction?: {
      active: boolean
      kind: 'delete' | 'archive' | 'markRead' | 'markUnread' | 'move' | null
      processed: number
      total: number
    }
    isLoading: boolean
    hasSearchQuery: boolean
    query: string
    searchMessages: (accountId: string, query: string) => Promise<void>
    setSyncStatus: (status: MailboxBundle['syncStatus']) => void
    setMessages: (messages: MailMessage[]) => void
    setMailboxBundle: (bundle: MailboxBundle, folderId: string | null) => void
  }
  mailboxesStore: {
    currentFolderId: string | null
    currentFolder: { kind: string; name: string } | null
    setFolders: (folders: MailboxBundle['folders']) => void
  }
}

const MAILBOX_CACHE_WINDOW_MS = 1200
export const useMailMailboxView = ({
  t,
  isSyncing,
  selectedLabel,
  messagesStore,
  mailboxesStore,
}: UseMailMailboxViewOptions) => {
  type RefreshMailboxOptions = {
    reloadLabels?: boolean
  }

  const sidebarLabels = ref<MailLabel[]>([])
  const currentMailboxBundle = ref<MailboxBundle | null>(null)
  const loadingStage = ref<LoadingStage>('idle')
  const mailboxRequestGeneration = ref(0)

  let mailboxLoadPromise: Promise<MailboxBundle> | null = null
  let mailboxLoadAccountId: string | null = null
  let labelsLoadPromise: Promise<MailLabel[]> | null = null
  let labelsLoadAccountId: string | null = null
  let lastMailboxLoadedAt = 0
  let refreshMailboxPromise: Promise<void> | null = null
  let refreshMailboxPending = false
  let refreshMailboxNeedsLabels = false
  const batchAction = computed(() => messagesStore.batchAction)
  const batchProgress = computed(() => {
    const current = batchAction.value
    if (!current?.active || current.total <= 0) {
      return 0
    }

    return Math.max(8, Math.round((current.processed / current.total) * 100))
  })

  const loadingBarActive = computed(() =>
    isSyncing.value
      || messagesStore.isLoading
      || loadingStage.value !== 'idle'
      || Boolean(batchAction.value?.active),
  )

  const loadingBarProgress = computed(() => {
    if (batchAction.value?.active) {
      return batchProgress.value
    }

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
    if (batchAction.value?.active) {
      switch (batchAction.value.kind) {
        case 'delete':
          return t('shell.deletingMessagesProgress', batchAction.value)
        case 'archive':
          return t('shell.archivingMessagesProgress', batchAction.value)
        case 'markRead':
          return t('shell.markingMessagesReadProgress', batchAction.value)
        case 'markUnread':
          return t('shell.markingMessagesUnreadProgress', batchAction.value)
        case 'move':
          return t('shell.movingMessagesProgress', batchAction.value)
        default:
          return ''
      }
    }

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

    if (
      !force
      && currentMailboxBundle.value
      && mailboxLoadAccountId === accountId
      && now - lastMailboxLoadedAt < MAILBOX_CACHE_WINDOW_MS
    ) {
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

    if (
      !force
      && sidebarLabels.value.length > 0
      && labelsLoadAccountId === accountId
    ) {
      return sidebarLabels.value
    }

    if (!force && labelsLoadPromise && labelsLoadAccountId === accountId) {
      return labelsLoadPromise
    }

    labelsLoadAccountId = accountId
    labelsLoadPromise = mailRepository.listLabels(accountId)
      .then((labels) => {
        sidebarLabels.value = labels
        return labels
      })
      .finally(() => {
        labelsLoadPromise = null
      })

    return labelsLoadPromise
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

  const refreshMailbox = async (accountId: string | null, options?: RefreshMailboxOptions) => {
    if (!accountId) {
      return
    }

    refreshMailboxNeedsLabels = refreshMailboxNeedsLabels || Boolean(options?.reloadLabels)

    if (refreshMailboxPromise) {
      refreshMailboxPending = true
      return refreshMailboxPromise
    }

    refreshMailboxPromise = (async () => {
      do {
        if (!accountId) {
          refreshMailboxPending = false
          break
        }
        refreshMailboxPending = false
        mailboxLoadPromise = null
        lastMailboxLoadedAt = 0
        if (refreshMailboxNeedsLabels) {
          labelsLoadPromise = null
          labelsLoadAccountId = null
        }
        await loadMailbox(accountId)
        refreshMailboxNeedsLabels = false
      } while (refreshMailboxPending)
    })().finally(() => {
      refreshMailboxPromise = null
      refreshMailboxNeedsLabels = false
    })

    return refreshMailboxPromise
  }

  const clearMailboxCaches = () => {
    mailboxLoadPromise = null
    mailboxLoadAccountId = null
    labelsLoadPromise = null
    labelsLoadAccountId = null
    lastMailboxLoadedAt = 0
  }

  return {
    applyMailboxView,
    clearMailboxCaches,
    currentMailboxBundle,
    fetchAccountLabels,
    fetchMailboxBundle,
    loadMailbox,
    loadingBarActive,
    loadingBarLabel,
    loadingBarProgress,
    loadingStage,
    refreshMailbox,
    setLoadingStage,
    sidebarLabels,
  }
}
