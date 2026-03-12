import type { MailRepository } from '@/shared/mail/mailRepository'
import type { MailstackBridge } from '@/shared/mail/bridge'

const getBridge = (): MailstackBridge => {
  if (!window.mailstack) {
    throw new Error('Mailstack bridge is not available. Use the Electron desktop target or the explicit web mock entrypoint.')
  }

  return window.mailstack
}

export const electronMailRepository: MailRepository = {
  listAccounts: () => getBridge().listAccounts(),
  createAccount: (draft) => getBridge().createAccount(draft),
  testAccountConnection: (draft) => getBridge().testAccountConnection(draft),
  listFolders: (accountId) => getBridge().listFolders(accountId),
  listMessages: (accountId, folderId) => getBridge().listMessages(accountId, folderId),
  getMessage: (accountId, messageId) => getBridge().getMessage(accountId, messageId),
  saveDraft: (draft) => getBridge().saveDraft(draft),
  sendMessage: (draft) => getBridge().sendMessage(draft),
  toggleStar: (accountId, messageId) => getBridge().toggleStar(accountId, messageId),
  toggleRead: (accountId, messageId) => getBridge().toggleRead(accountId, messageId),
  archiveMessage: (accountId, messageId) => getBridge().archiveMessage(accountId, messageId),
  restoreMessage: (accountId, messageId) => getBridge().restoreMessage(accountId, messageId),
  moveMessage: (accountId, messageId, folderId) => getBridge().moveMessage(accountId, messageId, folderId),
  deleteMessage: (accountId, messageId) => getBridge().deleteMessage(accountId, messageId),
  syncAccount: (accountId) => getBridge().syncAccount(accountId),
  getMailboxBundle: (accountId) => getBridge().getMailboxBundle(accountId),
}
