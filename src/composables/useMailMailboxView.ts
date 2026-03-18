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

const MAILBOX_CACHE_WINDOW_MS = 12_000
const MAILBOX_CACHE_MAX_AGE_MS = 10 * 60_000
const MAILBOX_BACKGROUND_REFRESH_COOLDOWN_MS = 45_000
const MAILBOX_PREWARM_CONCURRENCY = 2
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

  const mailboxBundleCache = new Map<string, MailboxBundle>()
  const mailboxLoadPromises = new Map<string, Promise<MailboxBundle>>()
  const mailboxLoadedAt = new Map<string, number>()
  const labelsCache = new Map<string, MailLabel[]>()
  const labelsLoadPromises = new Map<string, Promise<MailLabel[]>>()
  const mailboxBackgroundRefreshAt = new Map<string, number>()
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

  const isMailboxCacheFresh = (accountId: string) =>
    Date.now() - (mailboxLoadedAt.get(accountId) ?? 0) < MAILBOX_CACHE_WINDOW_MS

  const hasUsableMailboxCache = (accountId: string) =>
    Date.now() - (mailboxLoadedAt.get(accountId) ?? 0) < MAILBOX_CACHE_MAX_AGE_MS

  const shouldRunBackgroundRefresh = (accountId: string) =>
    Date.now() - (mailboxBackgroundRefreshAt.get(accountId) ?? 0) >= MAILBOX_BACKGROUND_REFRESH_COOLDOWN_MS

  const fetchMailboxBundle = async (accountId: string, options?: { force?: boolean; silent?: boolean }) => {
    const force = options?.force ?? false
    const silent = options?.silent ?? false
    const cachedBundle = mailboxBundleCache.get(accountId)

    if (!force && cachedBundle) {
      if (!silent) {
        currentMailboxBundle.value = cachedBundle
      }
      return cachedBundle
    }

    const inFlightRequest = mailboxLoadPromises.get(accountId)
    if (!force && inFlightRequest) {
      return inFlightRequest
    }

    const request = mailRepository.getMailboxBundle(accountId)
      .then((bundle) => {
        mailboxBundleCache.set(accountId, bundle)
        mailboxLoadedAt.set(accountId, Date.now())
        if (!silent) {
          currentMailboxBundle.value = bundle
        }
        return bundle
      })
      .finally(() => {
        mailboxLoadPromises.delete(accountId)
      })

    mailboxLoadPromises.set(accountId, request)
    return request
  }

  const refreshMailboxCacheSilently = async (accountId: string) => {
    if (!shouldRunBackgroundRefresh(accountId)) {
      return
    }

    mailboxBackgroundRefreshAt.set(accountId, Date.now())

    try {
      const [bundle, labels] = await Promise.all([
        fetchMailboxBundle(accountId, { force: true, silent: true }),
        fetchAccountLabels(accountId, { force: true, silent: true }),
      ])

      if (currentMailboxBundle.value?.accountId === accountId) {
        currentMailboxBundle.value = bundle
        sidebarLabels.value = labels
        await applyMailboxView(accountId, bundle)
      }
    } catch {
      // Ignore background refresh failures to preserve instant account switching.
    }
  }

  const fetchAccountLabels = async (accountId: string, options?: { force?: boolean; silent?: boolean }) => {
    const force = options?.force ?? false
    const silent = options?.silent ?? false
    const hasCachedLabels = labelsCache.has(accountId)
    const cachedLabels = labelsCache.get(accountId) ?? []

    if (
      !force
      && hasCachedLabels
    ) {
      if (!silent) {
        sidebarLabels.value = cachedLabels
      }
      return cachedLabels
    }

    const inFlightRequest = labelsLoadPromises.get(accountId)
    if (!force && inFlightRequest) {
      return inFlightRequest
    }

    const request = mailRepository.listLabels(accountId)
      .then((labels) => {
        labelsCache.set(accountId, labels)
        if (!silent) {
          sidebarLabels.value = labels
        }
        return labels
      })
      .finally(() => {
        labelsLoadPromises.delete(accountId)
      })

    labelsLoadPromises.set(accountId, request)
    return request
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

  const loadMailbox = async (accountId: string, options?: { force?: boolean; reloadLabels?: boolean }) => {
    messagesStore.isLoading = true

    try {
      setLoadingStage('fetching')
      const shouldRefreshStaleCache =
        !options?.force
        && mailboxBundleCache.has(accountId)
        && !isMailboxCacheFresh(accountId)
        && shouldRunBackgroundRefresh(accountId)
      const bundle = await fetchMailboxBundle(accountId, { force: options?.force ?? false })
      if (options?.reloadLabels) {
        await fetchAccountLabels(accountId, { force: true })
      }
      await applyMailboxView(accountId, bundle)
      setLoadingStage('finalizing')
      if (shouldRefreshStaleCache) {
        void refreshMailboxCacheSilently(accountId)
      }
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
        mailboxLoadPromises.delete(accountId)
        mailboxLoadedAt.delete(accountId)
        mailboxBundleCache.delete(accountId)
        if (refreshMailboxNeedsLabels) {
          labelsLoadPromises.delete(accountId)
          labelsCache.delete(accountId)
        }
        await loadMailbox(accountId, {
          force: true,
          reloadLabels: refreshMailboxNeedsLabels,
        })
        refreshMailboxNeedsLabels = false
      } while (refreshMailboxPending)
    })().finally(() => {
      refreshMailboxPromise = null
      refreshMailboxNeedsLabels = false
    })

    return refreshMailboxPromise
  }

  const clearMailboxCaches = (accountId?: string) => {
    if (accountId) {
      mailboxLoadPromises.delete(accountId)
      mailboxLoadedAt.delete(accountId)
      mailboxBackgroundRefreshAt.delete(accountId)
      mailboxBundleCache.delete(accountId)
      labelsLoadPromises.delete(accountId)
      labelsCache.delete(accountId)
      return
    }

    mailboxLoadPromises.clear()
    mailboxLoadedAt.clear()
    mailboxBackgroundRefreshAt.clear()
    mailboxBundleCache.clear()
    labelsLoadPromises.clear()
    labelsCache.clear()
  }

  const prewarmMailboxCaches = async (accountIds: string[], currentAccountId?: string | null) => {
    const targets = accountIds.filter((accountId) =>
      accountId
      && accountId !== currentAccountId
      && (!hasUsableMailboxCache(accountId) || !labelsCache.has(accountId)),
    )
    if (targets.length === 0) {
      return
    }

    const queue = targets.slice()
    const concurrency = Math.min(MAILBOX_PREWARM_CONCURRENCY, queue.length)
    await Promise.all(
      Array.from({ length: concurrency }, async () => {
        while (queue.length > 0) {
          const accountId = queue.shift()
          if (!accountId) {
            return
          }

          try {
            await Promise.all([
              fetchMailboxBundle(accountId, { silent: true }),
              fetchAccountLabels(accountId, { silent: true }),
            ])
          } catch {
            // Ignore background prewarm failures and keep the active mailbox responsive.
          }
        }
      }),
    )
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
    prewarmMailboxCaches,
    refreshMailbox,
    setLoadingStage,
    sidebarLabels,
  }
}
