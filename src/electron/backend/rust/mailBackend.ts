import type { MailBackend } from '../mailBackend'
import { authorizeOAuth } from '../oauth'
import { invokeRustBackend, shutdownRustBackend } from './process'

const inFlightSyncs = new Map<string, Promise<Awaited<ReturnType<MailBackend['syncAccount']>>>>()

const syncAccountDeduped: MailBackend['syncAccount'] = (accountId) => {
  const existing = inFlightSyncs.get(accountId)
  if (existing) {
    return existing
  }

  const promise = invokeRustBackend('syncAccount', { accountId }).finally(() => {
    if (inFlightSyncs.get(accountId) === promise) {
      inFlightSyncs.delete(accountId)
    }
  })
  inFlightSyncs.set(accountId, promise)
  return promise
}

export const rustMailBackend: MailBackend = {
  listAccounts: () => invokeRustBackend('listAccounts'),
  createAccount: (draft) => invokeRustBackend('createAccount', draft),
  testAccountConnection: (draft) => invokeRustBackend('testAccountConnection', draft),
  listFolders: (accountId) => invokeRustBackend('listFolders', { accountId }),
  listMessages: (accountId, folderId) => invokeRustBackend('listMessages', { accountId, folderId }),
  searchMessages: (accountId, query) => invokeRustBackend('searchMessages', { accountId, query }),
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
  syncAccount: syncAccountDeduped,
  getMailboxBundle: (accountId) => invokeRustBackend('getMailboxBundle', { accountId }),
  getAttachmentContent: (accountId, messageId, attachmentId) => invokeRustBackend('getAttachmentContent', { accountId, messageId, attachmentId }),
  getAccountConfig: (accountId) => invokeRustBackend('getAccountConfig', { accountId }),
  updateAccount: (accountId, draft) => invokeRustBackend('updateAccount', { accountId, draft }),
  listOAuthProviders: () => invokeRustBackend('listOAuthProviders'),
  authorizeOAuth,
  listContacts: (groupId) => invokeRustBackend('listContacts', { groupId }),
  createContact: (contact) => invokeRustBackend('createContact', contact),
  updateContact: (contactId, contact) => invokeRustBackend('updateContact', { contactId, contact }),
  deleteContact: (contactId) => invokeRustBackend('deleteContact', { contactId }),
  searchContacts: (query) => invokeRustBackend('searchContacts', { query }),
  listContactGroups: () => invokeRustBackend('listContactGroups'),
  createContactGroup: (name) => invokeRustBackend('createContactGroup', { name }),
  updateContactGroup: (groupId, name) => invokeRustBackend('updateContactGroup', { groupId, name }),
  deleteContactGroup: (groupId) => invokeRustBackend('deleteContactGroup', { groupId }),
  uploadContactAvatar: (contactId, dataBase64, mimeType) => invokeRustBackend('uploadContactAvatar', { contactId, dataBase64, mimeType }),
  deleteContactAvatar: (contactId) => invokeRustBackend('deleteContactAvatar', { contactId }),
  getContactAvatar: (contactId) => invokeRustBackend('getContactAvatar', { contactId }),
  getSecurityStatus: () => invokeRustBackend('getSecurityStatus'),
  unlockStorage: (password) => invokeRustBackend('unlockStorage', { password }),
  setMasterPassword: (currentPassword, newPassword) => invokeRustBackend('setMasterPassword', { currentPassword, newPassword }),
  clearMasterPassword: (currentPassword) => invokeRustBackend('clearMasterPassword', { currentPassword }),
  lockCurrentSession: async () => {
    await shutdownRustBackend()
    return invokeRustBackend('getSecurityStatus')
  },
  getStorageDir: () => invokeRustBackend('getStorageDir'),
}
