import type { Ref } from 'vue'
import type { MailMessage, MailboxFolder } from '@/types/mail'

interface MessageStoreLike {
  selectedMessageId: string | null
  selectedIds: Set<string>
  messages: MailMessage[]
  deleteMessage: (accountId: string, messageId: string) => Promise<void>
  archiveMessage: (accountId: string, messageId: string) => Promise<void>
  restoreMessage: (accountId: string, messageId: string) => Promise<void>
  moveMessage: (accountId: string, messageId: string, folderId: string) => Promise<void>
  markAllRead: (accountId: string, folderId: string) => Promise<void>
  batchDelete: (accountId: string) => Promise<void>
  batchArchive: (accountId: string) => Promise<void>
  batchToggleRead: (accountId: string, isRead: boolean) => Promise<void>
  batchMove: (accountId: string, folderId: string) => Promise<void>
  toggleRead: (accountId: string, messageId: string) => Promise<void>
}

interface MailboxStoreLike {
  currentFolderId: string | null
  currentFolder: MailboxFolder | null
  folders: MailboxFolder[]
  adjustUnread: (folderId: string, delta: number) => void
  error: string | null
  createFolder: (accountId: string, name: string) => Promise<unknown>
  renameFolder: (accountId: string, folderId: string, name: string) => Promise<unknown>
  deleteFolder: (accountId: string, folderId: string) => Promise<unknown>
}

interface UseMailMessageActionsOptions {
  t: (key: string, params?: Record<string, unknown>) => string
  currentAccountId: Ref<string | null>
  messagesStore: MessageStoreLike
  mailboxesStore: MailboxStoreLike
  refreshMailbox: (options?: { reloadLabels?: boolean }) => Promise<void>
  performUndoable: (label: string, undoFn: () => Promise<void>) => void
  applyCachedReadState: (messageIds: Iterable<string>, isRead: boolean) => void
  adjustUnreadCountsForMessages: (
    messages: Array<Pick<MailMessage, 'folderId' | 'isRead'>>,
    nextIsRead: boolean,
  ) => void
  patchCachedMessage: (messageId: string, updater: (message: MailMessage) => MailMessage) => void
}

export const useMailMessageActions = ({
  t,
  currentAccountId,
  messagesStore,
  mailboxesStore,
  refreshMailbox,
  performUndoable,
  applyCachedReadState,
  adjustUnreadCountsForMessages,
  patchCachedMessage,
}: UseMailMessageActionsOptions) => {
  const findMessage = (messageId: string) =>
    messagesStore.messages.find((message) => message.id === messageId)

  const getFolderIdByKind = (kind: 'inbox' | 'junk') =>
    mailboxesStore.folders.find((folder) => folder.kind === kind)?.id ?? null

  const restoreMessageFromJunk = async (messageId: string) => {
    if (!currentAccountId.value) return
    const inboxFolderId = getFolderIdByKind('inbox')
    if (!inboxFolderId) return

    await messagesStore.moveMessage(currentAccountId.value, messageId, inboxFolderId)
    await refreshMailbox()
  }

  const markMessageSpam = async (messageId: string) => {
    if (!currentAccountId.value) return
    const junkFolderId = getFolderIdByKind('junk')
    const originalFolderId = findMessage(messageId)?.folderId ?? mailboxesStore.currentFolderId
    if (!junkFolderId || !originalFolderId || originalFolderId === junkFolderId) return

    const accountId = currentAccountId.value
    await messagesStore.moveMessage(accountId, messageId, junkFolderId)
    await refreshMailbox()
    performUndoable(t('shell.messageMoved'), async () => {
      await messagesStore.moveMessage(accountId, messageId, originalFolderId)
      await refreshMailbox()
    })
  }

  const confirmDeleteCurrentMessage = async () => {
    if (!currentAccountId.value || !messagesStore.selectedMessageId) return

    const accountId = currentAccountId.value
    const messageId = messagesStore.selectedMessageId
    await messagesStore.deleteMessage(accountId, messageId)
    await refreshMailbox()
    performUndoable(t('shell.messageDeleted'), async () => {
      await messagesStore.restoreMessage(accountId, messageId)
      await refreshMailbox()
    })
  }

  const archiveCurrentMessage = async () => {
    if (!currentAccountId.value || !messagesStore.selectedMessageId) return

    const accountId = currentAccountId.value
    const messageId = messagesStore.selectedMessageId
    await messagesStore.archiveMessage(accountId, messageId)
    await refreshMailbox()
    performUndoable(t('shell.messageArchived'), async () => {
      await messagesStore.restoreMessage(accountId, messageId)
      await refreshMailbox()
    })
  }

  const restoreCurrentMessage = async () => {
    if (!currentAccountId.value || !messagesStore.selectedMessageId) return

    if (mailboxesStore.currentFolder?.kind === 'junk') {
      await restoreMessageFromJunk(messagesStore.selectedMessageId)
      return
    }

    await messagesStore.restoreMessage(currentAccountId.value, messagesStore.selectedMessageId)
    await refreshMailbox()
  }

  const moveCurrentMessage = async (folderId: string) => {
    if (!currentAccountId.value || !messagesStore.selectedMessageId) return

    const accountId = currentAccountId.value
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

  const markCurrentMessageSpam = async () => {
    if (!messagesStore.selectedMessageId) return
    await markMessageSpam(messagesStore.selectedMessageId)
  }

  const handleMarkAllRead = async () => {
    if (!currentAccountId.value || !mailboxesStore.currentFolderId) return

    const unreadMessages = messagesStore.messages
      .filter((message) => !message.isRead)
      .map((message) => ({ folderId: message.folderId, isRead: message.isRead }))
    await messagesStore.markAllRead(currentAccountId.value, mailboxesStore.currentFolderId)
    adjustUnreadCountsForMessages(unreadMessages, true)
    applyCachedReadState(messagesStore.messages.map((message) => message.id), true)
  }

  const handleCreateFolder = async (name: string) => {
    if (!currentAccountId.value) return

    try {
      await mailboxesStore.createFolder(currentAccountId.value, name)
      await refreshMailbox()
    } catch (error) {
      mailboxesStore.error = error instanceof Error ? error.message : 'Unable to create folder'
    }
  }

  const handleRenameFolder = async (folderId: string, name: string) => {
    if (!currentAccountId.value) return

    try {
      await mailboxesStore.renameFolder(currentAccountId.value, folderId, name)
      await refreshMailbox()
    } catch (error) {
      mailboxesStore.error = error instanceof Error ? error.message : 'Unable to rename folder'
    }
  }

  const handleDeleteFolder = async (folderId: string) => {
    if (!currentAccountId.value) return

    try {
      await mailboxesStore.deleteFolder(currentAccountId.value, folderId)
      await refreshMailbox()
    } catch (error) {
      mailboxesStore.error = error instanceof Error ? error.message : 'Unable to delete folder'
    }
  }

  const handleBatchDelete = async () => {
    if (!currentAccountId.value) return
    const accountId = currentAccountId.value
    const ids = [...messagesStore.selectedIds]
    await messagesStore.batchDelete(accountId)
    await refreshMailbox()
    performUndoable(t('shell.messagesDeleted', { count: ids.length }), async () => {
      for (const id of ids) await messagesStore.restoreMessage(accountId, id)
      await refreshMailbox()
    })
  }

  const handleBatchArchive = async () => {
    if (!currentAccountId.value) return
    const accountId = currentAccountId.value
    const ids = [...messagesStore.selectedIds]
    await messagesStore.batchArchive(accountId)
    await refreshMailbox()
    performUndoable(t('shell.messagesArchived', { count: ids.length }), async () => {
      for (const id of ids) await messagesStore.restoreMessage(accountId, id)
      await refreshMailbox()
    })
  }

  const handleBatchMarkRead = async () => {
    if (!currentAccountId.value) return
    const selectedIds = [...messagesStore.selectedIds]
    const affectedMessages = selectedIds
      .map((id) => messagesStore.messages.find((message) => message.id === id))
      .filter((message): message is NonNullable<typeof message> => Boolean(message))
    await messagesStore.batchToggleRead(currentAccountId.value, true)
    adjustUnreadCountsForMessages(affectedMessages, true)
    applyCachedReadState(selectedIds, true)
  }

  const handleBatchMarkUnread = async () => {
    if (!currentAccountId.value) return
    const selectedIds = [...messagesStore.selectedIds]
    const affectedMessages = selectedIds
      .map((id) => messagesStore.messages.find((message) => message.id === id))
      .filter((message): message is NonNullable<typeof message> => Boolean(message))
    await messagesStore.batchToggleRead(currentAccountId.value, false)
    adjustUnreadCountsForMessages(affectedMessages, false)
    applyCachedReadState(selectedIds, false)
  }

  const handleBatchMove = async (folderId: string) => {
    if (!currentAccountId.value) return
    const accountId = currentAccountId.value
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

  const handleContextToggleRead = async (messageId: string) => {
    if (!currentAccountId.value) return
    const before = messagesStore.messages.find((message) => message.id === messageId)
    await messagesStore.toggleRead(currentAccountId.value, messageId)
    const after = messagesStore.messages.find((message) => message.id === messageId)
    if (before && after) {
      mailboxesStore.adjustUnread(before.folderId, after.isRead === before.isRead ? 0 : (after.isRead ? -1 : 1))
      patchCachedMessage(messageId, () => after)
    }
  }

  const handleContextArchive = async (messageId: string) => {
    if (!currentAccountId.value) return
    const accountId = currentAccountId.value
    await messagesStore.archiveMessage(accountId, messageId)
    await refreshMailbox()
    performUndoable(t('shell.messageArchived'), async () => {
      await messagesStore.restoreMessage(accountId, messageId)
      await refreshMailbox()
    })
  }

  const handleContextRestore = async (messageId: string) => {
    if (!currentAccountId.value) return
    if (mailboxesStore.currentFolder?.kind === 'junk') {
      await restoreMessageFromJunk(messageId)
      return
    }
    await messagesStore.restoreMessage(currentAccountId.value, messageId)
    await refreshMailbox()
  }

  const handleContextMarkSpam = async (messageId: string) => {
    await markMessageSpam(messageId)
  }

  const handleContextDelete = async (messageId: string) => {
    if (!currentAccountId.value) return
    const accountId = currentAccountId.value
    await messagesStore.deleteMessage(accountId, messageId)
    await refreshMailbox()
    performUndoable(t('shell.messageDeleted'), async () => {
      await messagesStore.restoreMessage(accountId, messageId)
      await refreshMailbox()
    })
  }

  const handleContextMove = async (messageId: string, folderId: string) => {
    if (!currentAccountId.value) return
    const accountId = currentAccountId.value
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

  return {
    archiveCurrentMessage,
    confirmDeleteCurrentMessage,
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
  }
}
