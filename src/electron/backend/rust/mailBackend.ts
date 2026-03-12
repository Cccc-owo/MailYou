import type { MailBackend } from '../mailBackend'
import { invokeRustBackend } from './process'

export const rustMailBackend: MailBackend = {
  listAccounts: () => invokeRustBackend('listAccounts'),
  createAccount: (draft) => invokeRustBackend('createAccount', draft),
  listFolders: (accountId) => invokeRustBackend('listFolders', { accountId }),
  listMessages: (accountId, folderId) => invokeRustBackend('listMessages', { accountId, folderId }),
  getMessage: (accountId, messageId) => invokeRustBackend('getMessage', { accountId, messageId }),
  saveDraft: (draft) => invokeRustBackend('saveDraft', draft),
  sendMessage: (draft) => invokeRustBackend('sendMessage', draft),
  toggleStar: (accountId, messageId) => invokeRustBackend('toggleStar', { accountId, messageId }),
  toggleRead: (accountId, messageId) => invokeRustBackend('toggleRead', { accountId, messageId }),
  deleteMessage: (accountId, messageId) => invokeRustBackend('deleteMessage', { accountId, messageId }),
  syncAccount: (accountId) => invokeRustBackend('syncAccount', { accountId }),
  getMailboxBundle: (accountId) => invokeRustBackend('getMailboxBundle', { accountId }),
}
