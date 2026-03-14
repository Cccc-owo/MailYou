import type { MailBackend } from '../mailBackend'
import { invokeRustBackend } from './process'

export const rustMailBackend: MailBackend = {
  listAccounts: () => invokeRustBackend('listAccounts'),
  createAccount: (draft) => invokeRustBackend('createAccount', draft),
  testAccountConnection: (draft) => invokeRustBackend('testAccountConnection', draft),
  listFolders: (accountId) => invokeRustBackend('listFolders', { accountId }),
  listMessages: (accountId, folderId) => invokeRustBackend('listMessages', { accountId, folderId }),
  getMessage: (accountId, messageId) => invokeRustBackend('getMessage', { accountId, messageId }),
  saveDraft: (draft) => invokeRustBackend('saveDraft', draft),
  sendMessage: (draft) => invokeRustBackend('sendMessage', draft),
  toggleStar: (accountId, messageId) => invokeRustBackend('toggleStar', { accountId, messageId }),
  toggleRead: (accountId, messageId) => invokeRustBackend('toggleRead', { accountId, messageId }),
  archiveMessage: (accountId, messageId) => invokeRustBackend('archiveMessage', { accountId, messageId }),
  restoreMessage: (accountId, messageId) => invokeRustBackend('restoreMessage', { accountId, messageId }),
  moveMessage: (accountId, messageId, folderId) => invokeRustBackend('moveMessage', { accountId, messageId, folderId }),
  markAllRead: (accountId, folderId) => invokeRustBackend('markAllRead', { accountId, folderId }),
  deleteMessage: (accountId, messageId) => invokeRustBackend('deleteMessage', { accountId, messageId }),
  deleteAccount: (accountId) => invokeRustBackend('deleteAccount', { accountId }),
  syncAccount: (accountId) => invokeRustBackend('syncAccount', { accountId }),
  getMailboxBundle: (accountId) => invokeRustBackend('getMailboxBundle', { accountId }),
  getAttachmentContent: (accountId, messageId, attachmentId) => invokeRustBackend('getAttachmentContent', { accountId, messageId, attachmentId }),
  getAccountConfig: (accountId) => invokeRustBackend('getAccountConfig', { accountId }),
  updateAccount: (accountId, draft) => invokeRustBackend('updateAccount', { accountId, draft }),
}
