import { computed, ref } from 'vue'
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

  return folderMessages.sort((a, b) => b.receivedAt.localeCompare(a.receivedAt))
}

export const useMessagesStore = defineStore('messages', () => {
  const messages = ref<MailMessage[]>([])
  const selectedMessageId = ref<string | null>(null)
  const isLoading = ref(false)
  const query = ref('')
  const syncStatus = ref<SyncStatus | null>(null)
  const error = ref<string | null>(null)

  const filteredMessages = computed(() => {
    const search = query.value.trim().toLowerCase()

    if (!search) {
      return messages.value
    }

    return messages.value.filter((message) =>
      [message.subject, message.preview, message.from, message.fromEmail]
        .join(' ')
        .toLowerCase()
        .includes(search),
    )
  })

  const selectedMessage = computed(() =>
    messages.value.find((message) => message.id === selectedMessageId.value) ?? null,
  )

  const setSyncStatus = (value: SyncStatus | null) => {
    syncStatus.value = value
  }

  const setMessages = (nextMessages: MailMessage[]) => {
    messages.value = nextMessages
    selectedMessageId.value = nextMessages[0]?.id ?? null
  }

  const setMailboxBundle = (bundle: MailboxBundle, folderId: string | null) => {
    syncStatus.value = bundle.syncStatus
    setMessages(getMessagesForFolder(bundle.messages, bundle.folders, folderId))
  }

  const loadMessages = async (accountId: string, folderId: string) => {
    isLoading.value = true
    error.value = null

    try {
      setMessages(await mailRepository.listMessages(accountId, folderId))
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : 'Unable to load messages'
    } finally {
      isLoading.value = false
    }
  }

  const selectMessage = (messageId: string) => {
    selectedMessageId.value = messageId
  }

  const toggleStar = async (accountId: string, messageId: string) => {
    const updated = await mailRepository.toggleStar(accountId, messageId)

    if (!updated) {
      return
    }

    messages.value = messages.value.map((message) => (message.id === messageId ? updated : message))
  }

  const toggleRead = async (accountId: string, messageId: string) => {
    const updated = await mailRepository.toggleRead(accountId, messageId)

    if (!updated) {
      return
    }

    messages.value = messages.value.map((message) => (message.id === messageId ? updated : message))
  }

  const deleteMessage = async (accountId: string, messageId: string) => {
    await mailRepository.deleteMessage(accountId, messageId)
    messages.value = messages.value.filter((message) => message.id !== messageId)

    if (selectedMessageId.value === messageId) {
      selectedMessageId.value = messages.value[0]?.id ?? null
    }
  }

  const syncAccount = async (accountId: string) => {
    syncStatus.value = await mailRepository.syncAccount(accountId)
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
    setSyncStatus,
    setMessages,
    setMailboxBundle,
    loadMessages,
    selectMessage,
    toggleStar,
    toggleRead,
    deleteMessage,
    syncAccount,
  }
})
