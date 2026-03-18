import { computed, ref, shallowReactive } from 'vue'
import { defineStore } from 'pinia'
import { mailRepository } from '@/services/mail'
import type { MailMessage, MailThreadSummary, MailboxBundle, MailboxFolder, SyncStatus } from '@/types/mail'

const getMessagesForFolder = (
  allMessages: MailMessage[],
  folders: MailboxFolder[],
  folderId: string | null,
) => {
  if (!folderId) {
    return []
  }

  const folder = folders.find((item) => item.id === folderId)

  if (!folder) {
    return []
  }

  const folderMessages =
    folder.kind === 'starred'
      ? allMessages.filter((message) => message.accountId === folder.accountId && message.isStarred)
      : allMessages.filter(
          (message) => message.accountId === folder.accountId && message.folderId === folderId,
        )

  return folderMessages.sort((a, b) => new Date(b.receivedAt).getTime() - new Date(a.receivedAt).getTime())
}

export const useMessagesStore = defineStore('messages', () => {
  const BATCH_MUTATION_CHUNK_SIZE = 96
  const messages = ref<MailMessage[]>([])
  const selectedMessageId = ref<string | null>(null)
  const selectionAnchorId = ref<string | null>(null)
  const isLoading = ref(false)
  const query = ref('')
  const syncStatus = ref<SyncStatus | null>(null)
  const error = ref<string | null>(null)
  const selectedIds = shallowReactive(new Set<string>())
  const isMultiSelectMode = computed(() => selectedIds.size > 0)
  const batchAction = ref<{
    active: boolean
    kind: 'delete' | 'archive' | 'markRead' | 'markUnread' | 'move' | null
    processed: number
    total: number
  }>({
    active: false,
    kind: null,
    processed: 0,
    total: 0,
  })
  let loadGeneration = 0
  let inFlightListRequest:
    | {
        key: string
        promise: Promise<MailMessage[]>
      }
    | null = null
  let inFlightSearchRequest:
    | {
        key: string
        promise: Promise<MailMessage[]>
      }
    | null = null

  const sortByDate = (list: MailMessage[]) =>
    list.slice().sort((a, b) => new Date(b.receivedAt).getTime() - new Date(a.receivedAt).getTime())

  const runWithConcurrency = async <T>(
    items: T[],
    limit: number,
    worker: (item: T) => Promise<void>,
  ) => {
    const queue = items.slice()
    const failures: unknown[] = []
    const concurrency = Math.max(1, Math.min(limit, queue.length))

    await Promise.all(
      Array.from({ length: concurrency }, async () => {
        while (queue.length > 0) {
          const next = queue.shift()
          if (!next) {
            return
          }

          try {
            await worker(next)
          } catch (error) {
            failures.push(error)
          }
        }
      }),
    )

    return failures
  }

  const chunkItems = <T>(items: T[], size: number) => {
    const chunks: T[][] = []

    for (let index = 0; index < items.length; index += size) {
      chunks.push(items.slice(index, index + size))
    }

    return chunks
  }

  const filteredMessages = computed(() => {
    return sortByDate(messages.value)
  })

  const threadSummaries = computed<MailThreadSummary[]>(() => {
    const byThread = new Map<string, MailMessage[]>()

    for (const message of filteredMessages.value) {
      const thread = byThread.get(message.threadId)
      if (thread) {
        thread.push(message)
      } else {
        byThread.set(message.threadId, [message])
      }
    }

    return [...byThread.entries()]
      .map(([threadId, threadMessages]) => {
        const sorted = sortByDate(threadMessages)
        const participants = [...new Set(sorted.map((message) => message.from).filter(Boolean))]

        return {
          threadId,
          accountId: sorted[0].accountId,
          message: sorted[0],
          messageCount: sorted.length,
          unreadCount: sorted.filter((message) => !message.isRead).length,
          participants,
        }
      })
      .sort((a, b) =>
        new Date(b.message.receivedAt).getTime() - new Date(a.message.receivedAt).getTime(),
      )
  })

  const selectedMessage = computed(() =>
    messages.value.find((message) => message.id === selectedMessageId.value) ?? null,
  )

  const selectedThreadMessages = computed(() => {
    if (!selectedMessage.value) {
      return []
    }

    return messages.value
      .filter((message) => message.threadId === selectedMessage.value!.threadId)
      .slice()
      .sort((a, b) => new Date(a.receivedAt).getTime() - new Date(b.receivedAt).getTime())
  })

  const computeNextSelectedId = (removedId: string): string | null => {
    if (selectedMessageId.value !== removedId) return selectedMessageId.value
    const sorted = filteredMessages.value
    const idx = sorted.findIndex((m) => m.id === removedId)
    if (idx < 0) return null
    return sorted[idx + 1]?.id ?? sorted[idx - 1]?.id ?? null
  }

  const hasSearchQuery = computed(() => query.value.trim().length > 0)

  const setSyncStatus = (value: SyncStatus | null) => {
    syncStatus.value = value
  }

  const startBatchAction = (kind: NonNullable<typeof batchAction.value.kind>, total: number) => {
    batchAction.value = {
      active: total > 0,
      kind,
      processed: 0,
      total,
    }
  }

  const advanceBatchAction = () => {
    if (!batchAction.value.active) {
      return
    }

    batchAction.value = {
      ...batchAction.value,
      processed: Math.min(batchAction.value.processed + 1, batchAction.value.total),
    }
  }

  const finishBatchAction = () => {
    batchAction.value = {
      active: false,
      kind: null,
      processed: 0,
      total: 0,
    }
  }

  const setMessages = (nextMessages: MailMessage[]) => {
    messages.value = nextMessages

    // Preserve current selection if the message still exists in the new list
    if (selectedMessageId.value && nextMessages.some((m) => m.id === selectedMessageId.value)) {
      return
    }

    // Auto-select the most recent message by date
    const sorted = sortByDate(nextMessages)
    selectedMessageId.value = sorted[0]?.id ?? null
  }

  const setMailboxBundle = (bundle: MailboxBundle, folderId: string | null) => {
    syncStatus.value = bundle.syncStatus
    setMessages(getMessagesForFolder(bundle.messages, bundle.folders, folderId))
  }

  const loadMessages = async (accountId: string, folderId: string) => {
    isLoading.value = true
    const gen = ++loadGeneration
    const key = `${accountId}\n${folderId}`

    try {
      const result = await (
        inFlightListRequest?.key === key
          ? inFlightListRequest.promise
          : (() => {
              const promise = mailRepository.listMessages(accountId, folderId)
              inFlightListRequest = { key, promise }
              return promise.finally(() => {
                if (inFlightListRequest?.key === key) {
                  inFlightListRequest = null
                }
              })
            })()
      )
      if (gen !== loadGeneration) return // stale response, discard
      setMessages(result)
    } catch (loadError) {
      if (gen !== loadGeneration) return
      error.value = loadError instanceof Error ? loadError.message : 'Unable to load messages'
    } finally {
      if (gen === loadGeneration) {
        isLoading.value = false
      }
    }
  }

  const searchMessages = async (accountId: string, searchQuery: string) => {
    isLoading.value = true
    const gen = ++loadGeneration
    const normalizedQuery = searchQuery.trim()
    const key = `${accountId}\n${normalizedQuery}`

    try {
      const result = await (
        inFlightSearchRequest?.key === key
          ? inFlightSearchRequest.promise
          : (() => {
              const promise = mailRepository.searchMessages(accountId, normalizedQuery)
              inFlightSearchRequest = { key, promise }
              return promise.finally(() => {
                if (inFlightSearchRequest?.key === key) {
                  inFlightSearchRequest = null
                }
              })
            })()
      )
      if (gen !== loadGeneration) return
      setMessages(result)
    } catch (loadError) {
      if (gen !== loadGeneration) return
      error.value = loadError instanceof Error ? loadError.message : 'Unable to search messages'
    } finally {
      if (gen === loadGeneration) {
        isLoading.value = false
      }
    }
  }

  const selectMessage = (messageId: string) => {
    selectedMessageId.value = messageId
    selectionAnchorId.value = messageId
  }

  const getSelectionRangeIds = (targetMessageId: string) => {
    const anchorId = selectionAnchorId.value ?? selectedMessageId.value
    if (!anchorId) {
      return [targetMessageId]
    }

    const orderedIds = filteredMessages.value.map((message) => message.id)
    const anchorIndex = orderedIds.indexOf(anchorId)
    const targetIndex = orderedIds.indexOf(targetMessageId)

    if (anchorIndex === -1 || targetIndex === -1) {
      return [targetMessageId]
    }

    const start = Math.min(anchorIndex, targetIndex)
    const end = Math.max(anchorIndex, targetIndex)
    return orderedIds.slice(start, end + 1)
  }

  const selectMessageRange = (messageId: string) => {
    for (const id of getSelectionRangeIds(messageId)) {
      selectedIds.add(id)
    }
    selectedMessageId.value = messageId
  }

  const toggleStar = async (accountId: string, messageId: string) => {
    const original = messages.value.find((m) => m.id === messageId)
    if (!original) return
    const optimisticStar = !original.isStarred
    messages.value = messages.value.map((m) => (m.id === messageId ? { ...m, isStarred: optimisticStar } : m))

    try {
      const updated = await mailRepository.toggleStar(accountId, messageId)
      if (updated) {
        messages.value = messages.value.map((m) => (m.id === messageId ? updated : m))
      }
    } catch {
      messages.value = messages.value.map((m) => (m.id === messageId ? { ...m, isStarred: !optimisticStar } : m))
    }
  }

  const toggleRead = async (accountId: string, messageId: string) => {
    // Optimistic update
    const original = messages.value.find((m) => m.id === messageId)
    if (!original) return
    const optimisticRead = !original.isRead
    messages.value = messages.value.map((m) => (m.id === messageId ? { ...m, isRead: optimisticRead } : m))

    try {
      const updated = await mailRepository.toggleRead(accountId, messageId)
      if (updated) {
        messages.value = messages.value.map((m) => (m.id === messageId ? updated : m))
      }
    } catch {
      // Rollback on failure
      messages.value = messages.value.map((m) => (m.id === messageId ? { ...m, isRead: !optimisticRead } : m))
    }
  }

  const deleteMessage = async (accountId: string, messageId: string) => {
    const nextId = computeNextSelectedId(messageId)
    await mailRepository.deleteMessage(accountId, messageId)
    messages.value = messages.value.filter((message) => message.id !== messageId)
    selectedMessageId.value = nextId
  }

  const archiveMessage = async (accountId: string, messageId: string) => {
    const nextId = computeNextSelectedId(messageId)
    await mailRepository.archiveMessage(accountId, messageId)
    messages.value = messages.value.filter((message) => message.id !== messageId)
    selectedMessageId.value = nextId
  }

  const restoreMessage = async (accountId: string, messageId: string) => {
    const nextId = computeNextSelectedId(messageId)
    await mailRepository.restoreMessage(accountId, messageId)
    messages.value = messages.value.filter((message) => message.id !== messageId)
    selectedMessageId.value = nextId
  }

  const moveMessage = async (accountId: string, messageId: string, folderId: string) => {
    const nextId = computeNextSelectedId(messageId)
    await mailRepository.moveMessage(accountId, messageId, folderId)
    messages.value = messages.value.filter((message) => message.id !== messageId)
    selectedMessageId.value = nextId
  }

  const markAllRead = async (accountId: string, folderId: string) => {
    await mailRepository.markAllRead(accountId, folderId)
    messages.value = messages.value.map((message) => ({ ...message, isRead: true }))
  }

  // --- Batch selection ---
  const toggleSelection = (messageId: string, options?: { shiftKey?: boolean }) => {
    if (options?.shiftKey) {
      selectMessageRange(messageId)
      return
    }

    if (selectedIds.has(messageId)) {
      selectedIds.delete(messageId)
    } else {
      selectedIds.add(messageId)
    }

    selectionAnchorId.value = messageId
  }

  const selectAll = () => {
    for (const message of filteredMessages.value) {
      selectedIds.add(message.id)
    }
    selectionAnchorId.value = filteredMessages.value[0]?.id ?? null
  }

  const clearSelection = () => {
    selectedIds.clear()
    selectionAnchorId.value = null
  }

  const batchDelete = async (accountId: string) => {
    if (batchAction.value.active) {
      return
    }

    const ids = [...selectedIds]
    if (ids.length === 0) {
      return
    }
    const nextId = selectedMessageId.value && selectedIds.has(selectedMessageId.value)
      ? computeNextSelectedId(selectedMessageId.value)
      : selectedMessageId.value
    const snapshot = messages.value.filter((message) => selectedIds.has(message.id))
    const snapshotById = new Map(snapshot.map((message) => [message.id, message]))
    const succeeded = new Set<string>()
    startBatchAction('delete', ids.length)
    messages.value = messages.value.filter((message) => !selectedIds.has(message.id))
    selectedMessageId.value = selectedIds.has(selectedMessageId.value ?? '') ? nextId : selectedMessageId.value
    selectedIds.clear()
    const failures: unknown[] = []
    for (const batchIds of chunkItems(ids, BATCH_MUTATION_CHUNK_SIZE)) {
      try {
        await mailRepository.batchDeleteMessages(accountId, batchIds)
        batchIds.forEach((id) => {
          succeeded.add(id)
          advanceBatchAction()
        })
      } catch (error) {
        failures.push(error)
      }
    }
    if (failures.length > 0) {
      const firstFailure = failures[0]
      error.value = firstFailure instanceof Error ? firstFailure.message : 'Batch delete failed'
      const failedMessages = ids
        .filter((id) => !succeeded.has(id))
        .map((id) => snapshotById.get(id))
        .filter((message): message is MailMessage => Boolean(message))
      if (failedMessages.length > 0) {
        messages.value = sortByDate([...messages.value, ...failedMessages])
      }
    }
    finishBatchAction()
  }

  const batchArchive = async (accountId: string) => {
    if (batchAction.value.active) {
      return
    }

    const ids = [...selectedIds]
    if (ids.length === 0) {
      return
    }
    const nextId = selectedMessageId.value && selectedIds.has(selectedMessageId.value)
      ? computeNextSelectedId(selectedMessageId.value)
      : selectedMessageId.value
    const snapshot = messages.value.filter((message) => selectedIds.has(message.id))
    const snapshotById = new Map(snapshot.map((message) => [message.id, message]))
    const succeeded = new Set<string>()
    startBatchAction('archive', ids.length)
    messages.value = messages.value.filter((message) => !selectedIds.has(message.id))
    selectedMessageId.value = selectedIds.has(selectedMessageId.value ?? '') ? nextId : selectedMessageId.value
    selectedIds.clear()
    const failures = await runWithConcurrency(ids, 4, async (id) => {
      await mailRepository.archiveMessage(accountId, id)
      succeeded.add(id)
      advanceBatchAction()
    })
    if (failures.length > 0) {
      const firstFailure = failures[0]
      error.value = firstFailure instanceof Error ? firstFailure.message : 'Batch archive failed'
      const failedMessages = ids
        .filter((id) => !succeeded.has(id))
        .map((id) => snapshotById.get(id))
        .filter((message): message is MailMessage => Boolean(message))
      if (failedMessages.length > 0) {
        messages.value = sortByDate([...messages.value, ...failedMessages])
      }
    }
    finishBatchAction()
  }

  const batchToggleRead = async (accountId: string, markRead: boolean) => {
    if (batchAction.value.active) {
      return
    }

    const ids = [...selectedIds]
    if (ids.length === 0) {
      return
    }
    const succeeded = new Set<string>()
    const originalStates = new Map(
      ids
        .map((id) => messages.value.find((message) => message.id === id))
        .filter((message): message is MailMessage => Boolean(message))
        .map((message) => [message.id, message.isRead]),
    )
    startBatchAction(markRead ? 'markRead' : 'markUnread', ids.length)
    messages.value = messages.value.map((message) =>
      selectedIds.has(message.id) ? { ...message, isRead: markRead } : message,
    )
    selectedIds.clear()
    const failures: unknown[] = []
    for (const batchIds of chunkItems(
      ids.filter((id) => originalStates.get(id) !== undefined),
      BATCH_MUTATION_CHUNK_SIZE,
    )) {
      try {
        const actionableIds = batchIds.filter((id) => originalStates.get(id) !== markRead)
        if (actionableIds.length > 0) {
          await mailRepository.batchToggleRead(accountId, actionableIds, markRead)
        }
        batchIds.forEach((id) => {
          succeeded.add(id)
          advanceBatchAction()
        })
      } catch (error) {
        failures.push(error)
      }
    }
    if (failures.length > 0) {
      const firstFailure = failures[0]
      error.value = firstFailure instanceof Error ? firstFailure.message : 'Batch mark read failed'
      messages.value = messages.value.map((message) => {
        if (!originalStates.has(message.id) || succeeded.has(message.id)) {
          return message
        }

        return {
          ...message,
          isRead: originalStates.get(message.id) ?? message.isRead,
        }
      })
    }
    finishBatchAction()
  }

  const batchMove = async (accountId: string, folderId: string) => {
    if (batchAction.value.active) {
      return
    }

    const ids = [...selectedIds]
    if (ids.length === 0) {
      return
    }
    const nextId = selectedMessageId.value && selectedIds.has(selectedMessageId.value)
      ? computeNextSelectedId(selectedMessageId.value)
      : selectedMessageId.value
    const snapshot = messages.value.filter((message) => selectedIds.has(message.id))
    const snapshotById = new Map(snapshot.map((message) => [message.id, message]))
    const succeeded = new Set<string>()
    startBatchAction('move', ids.length)
    messages.value = messages.value.filter((message) => !selectedIds.has(message.id))
    selectedMessageId.value = selectedIds.has(selectedMessageId.value ?? '') ? nextId : selectedMessageId.value
    selectedIds.clear()
    const failures: unknown[] = []
    for (const batchIds of chunkItems(ids, BATCH_MUTATION_CHUNK_SIZE)) {
      try {
        await mailRepository.batchMoveMessages(accountId, batchIds, folderId)
        batchIds.forEach((id) => {
          succeeded.add(id)
          advanceBatchAction()
        })
      } catch (error) {
        failures.push(error)
      }
    }
    if (failures.length > 0) {
      const firstFailure = failures[0]
      error.value = firstFailure instanceof Error ? firstFailure.message : 'Batch move failed'
      const failedMessages = ids
        .filter((id) => !succeeded.has(id))
        .map((id) => snapshotById.get(id))
        .filter((message): message is MailMessage => Boolean(message))
      if (failedMessages.length > 0) {
        messages.value = sortByDate([...messages.value, ...failedMessages])
      }
    }
    finishBatchAction()
  }

  const syncAccount = async (accountId: string) => {
    error.value = null

    try {
      syncStatus.value = await mailRepository.syncAccount(accountId)
    } catch (syncError) {
      const message = syncError instanceof Error ? syncError.message : 'Unable to sync mailbox'
      error.value = message
      syncStatus.value = {
        accountId,
        state: 'error',
        message,
        updatedAt: new Date().toISOString(),
      }
    }
  }

  const clearError = () => {
    error.value = null
  }

  const clearAll = () => {
    messages.value = []
    selectedMessageId.value = null
    syncStatus.value = null
    error.value = null
    query.value = ''
    selectedIds.clear()
    selectionAnchorId.value = null
  }

  return {
    messages,
    filteredMessages,
    threadSummaries,
    selectedMessage,
    selectedThreadMessages,
    selectedMessageId,
    isLoading,
    query,
    syncStatus,
    error,
    batchAction,
    hasSearchQuery,
    selectedIds,
    isMultiSelectMode,
    setSyncStatus,
    setMessages,
    setMailboxBundle,
    loadMessages,
    searchMessages,
    selectMessage,
    selectMessageRange,
    toggleStar,
    toggleRead,
    deleteMessage,
    archiveMessage,
    restoreMessage,
    moveMessage,
    markAllRead,
    toggleSelection,
    selectAll,
    clearSelection,
    batchDelete,
    batchArchive,
    batchToggleRead,
    batchMove,
    syncAccount,
    clearError,
    clearAll,
  }
})
