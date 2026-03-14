import { computed, ref, shallowReactive } from 'vue'
import { defineStore } from 'pinia'
import { mailRepository } from '@/services/mail'
import type { MailMessage, MailboxBundle, MailboxFolder, SyncStatus } from '@/types/mail'

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
  const messages = ref<MailMessage[]>([])
  const selectedMessageId = ref<string | null>(null)
  const isLoading = ref(false)
  const query = ref('')
  const syncStatus = ref<SyncStatus | null>(null)
  const error = ref<string | null>(null)
  const selectedIds = shallowReactive(new Set<string>())
  const isMultiSelectMode = computed(() => selectedIds.size > 0)
  let loadGeneration = 0

  const sortByDate = (list: MailMessage[]) =>
    list.slice().sort((a, b) => new Date(b.receivedAt).getTime() - new Date(a.receivedAt).getTime())

  const filteredMessages = computed(() => {
    const search = query.value.trim().toLowerCase()

    if (!search) {
      return sortByDate(messages.value)
    }

    return sortByDate(
      messages.value.filter((message) => {
        const basicFields = [message.subject, message.preview, message.from, message.fromEmail]
          .join(' ')
          .toLowerCase()

        if (basicFields.includes(search)) {
          return true
        }

        // Search in body with HTML tags stripped
        const bodyText = message.body.replace(/<[^>]*>/g, ' ').toLowerCase()
        return bodyText.includes(search)
      }),
    )
  })

  const selectedMessage = computed(() =>
    messages.value.find((message) => message.id === selectedMessageId.value) ?? null,
  )

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
    error.value = null
    const gen = ++loadGeneration

    try {
      const result = await mailRepository.listMessages(accountId, folderId)
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

  const selectMessage = (messageId: string) => {
    selectedMessageId.value = messageId
  }

  const toggleStar = async (accountId: string, messageId: string) => {
    error.value = null
    const updated = await mailRepository.toggleStar(accountId, messageId)

    if (!updated) {
      return
    }

    messages.value = messages.value.map((message) => (message.id === messageId ? updated : message))
  }

  const toggleRead = async (accountId: string, messageId: string) => {
    error.value = null
    const updated = await mailRepository.toggleRead(accountId, messageId)

    if (!updated) {
      return
    }

    messages.value = messages.value.map((message) => (message.id === messageId ? updated : message))
  }

  const deleteMessage = async (accountId: string, messageId: string) => {
    error.value = null
    const nextId = computeNextSelectedId(messageId)
    await mailRepository.deleteMessage(accountId, messageId)
    messages.value = messages.value.filter((message) => message.id !== messageId)
    selectedMessageId.value = nextId
  }

  const archiveMessage = async (accountId: string, messageId: string) => {
    error.value = null
    const nextId = computeNextSelectedId(messageId)
    await mailRepository.archiveMessage(accountId, messageId)
    messages.value = messages.value.filter((message) => message.id !== messageId)
    selectedMessageId.value = nextId
  }

  const restoreMessage = async (accountId: string, messageId: string) => {
    error.value = null
    const nextId = computeNextSelectedId(messageId)
    await mailRepository.restoreMessage(accountId, messageId)
    messages.value = messages.value.filter((message) => message.id !== messageId)
    selectedMessageId.value = nextId
  }

  const moveMessage = async (accountId: string, messageId: string, folderId: string) => {
    error.value = null
    const nextId = computeNextSelectedId(messageId)
    await mailRepository.moveMessage(accountId, messageId, folderId)
    messages.value = messages.value.filter((message) => message.id !== messageId)
    selectedMessageId.value = nextId
  }

  const markAllRead = async (accountId: string, folderId: string) => {
    error.value = null
    await mailRepository.markAllRead(accountId, folderId)
    messages.value = messages.value.map((message) => ({ ...message, isRead: true }))
  }

  // --- Batch selection ---
  const toggleSelection = (messageId: string) => {
    if (selectedIds.has(messageId)) {
      selectedIds.delete(messageId)
    } else {
      selectedIds.add(messageId)
    }
  }

  const selectAll = () => {
    for (const message of filteredMessages.value) {
      selectedIds.add(message.id)
    }
  }

  const clearSelection = () => {
    selectedIds.clear()
  }

  const batchDelete = async (accountId: string) => {
    error.value = null
    const ids = [...selectedIds]
    const nextId = selectedMessageId.value && selectedIds.has(selectedMessageId.value)
      ? computeNextSelectedId(selectedMessageId.value)
      : selectedMessageId.value
    const succeeded = new Set<string>()
    try {
      for (const id of ids) {
        await mailRepository.deleteMessage(accountId, id)
        succeeded.add(id)
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Batch delete failed'
    }
    messages.value = messages.value.filter((m) => !succeeded.has(m.id))
    selectedMessageId.value = succeeded.has(selectedMessageId.value ?? '') ? nextId : selectedMessageId.value
    selectedIds.clear()
  }

  const batchArchive = async (accountId: string) => {
    error.value = null
    const ids = [...selectedIds]
    const nextId = selectedMessageId.value && selectedIds.has(selectedMessageId.value)
      ? computeNextSelectedId(selectedMessageId.value)
      : selectedMessageId.value
    const succeeded = new Set<string>()
    try {
      for (const id of ids) {
        await mailRepository.archiveMessage(accountId, id)
        succeeded.add(id)
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Batch archive failed'
    }
    messages.value = messages.value.filter((m) => !succeeded.has(m.id))
    selectedMessageId.value = succeeded.has(selectedMessageId.value ?? '') ? nextId : selectedMessageId.value
    selectedIds.clear()
  }

  const batchToggleRead = async (accountId: string, markRead: boolean) => {
    error.value = null
    const ids = [...selectedIds]
    const succeeded = new Set<string>()
    try {
      for (const id of ids) {
        const msg = messages.value.find((m) => m.id === id)
        if (msg && msg.isRead !== markRead) {
          await mailRepository.toggleRead(accountId, id)
        }
        succeeded.add(id)
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Batch mark read failed'
    }
    messages.value = messages.value.map((m) =>
      succeeded.has(m.id) ? { ...m, isRead: markRead } : m,
    )
    selectedIds.clear()
  }

  const batchMove = async (accountId: string, folderId: string) => {
    error.value = null
    const ids = [...selectedIds]
    const nextId = selectedMessageId.value && selectedIds.has(selectedMessageId.value)
      ? computeNextSelectedId(selectedMessageId.value)
      : selectedMessageId.value
    const succeeded = new Set<string>()
    try {
      for (const id of ids) {
        await mailRepository.moveMessage(accountId, id, folderId)
        succeeded.add(id)
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Batch move failed'
    }
    messages.value = messages.value.filter((m) => !succeeded.has(m.id))
    selectedMessageId.value = succeeded.has(selectedMessageId.value ?? '') ? nextId : selectedMessageId.value
    selectedIds.clear()
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
  }

  return {
    messages,
    filteredMessages,
    selectedMessage,
    selectedMessageId,
    isLoading,
    query,
    syncStatus,
    error,
    hasSearchQuery,
    selectedIds,
    isMultiSelectMode,
    setSyncStatus,
    setMessages,
    setMailboxBundle,
    loadMessages,
    selectMessage,
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
