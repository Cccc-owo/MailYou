import type { MailBackend } from '../mailBackend'
import { authorizeOAuth, handleOAuthCallbackUrl } from '../oauth'
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
  createFolder: (accountId, name) => invokeRustBackend('createFolder', { accountId, name }),
  renameFolder: (accountId, folderId, name) => invokeRustBackend('renameFolder', { accountId, folderId, name }),
  deleteFolder: (accountId, folderId) => invokeRustBackend('deleteFolder', { accountId, folderId }),
  listMessages: (accountId, folderId) => invokeRustBackend('listMessages', { accountId, folderId }),
  getDraft: (accountId, draftId) => invokeRustBackend('getDraft', { accountId, draftId }),
  searchMessages: (accountId, query) => invokeRustBackend('searchMessages', { accountId, query }),
  listLabels: (accountId) => invokeRustBackend('listLabels', { accountId }),
  getMessage: (accountId, messageId) => invokeRustBackend('getMessage', { accountId, messageId }),
  addLabel: (accountId, messageId, label) => invokeRustBackend('addLabel', { accountId, messageId, label }),
  removeLabel: (accountId, messageId, label) => invokeRustBackend('removeLabel', { accountId, messageId, label }),
  renameLabel: (accountId, label, newLabel) => invokeRustBackend('renameLabel', { accountId, label, newLabel }),
  deleteLabel: (accountId, label) => invokeRustBackend('deleteLabel', { accountId, label }),
  saveDraft: (draft) => invokeRustBackend('saveDraft', draft),
  sendMessage: (draft) => invokeRustBackend('sendMessage', draft),
  toggleStar: (accountId, messageId) => invokeRustBackend('toggleStar', { accountId, messageId }),
  toggleRead: (accountId, messageId) => invokeRustBackend('toggleRead', { accountId, messageId }),
  batchToggleRead: (accountId, messageIds, isRead) => invokeRustBackend('batchToggleRead', { accountId, messageIds, isRead }),
  archiveMessage: (accountId, messageId) => invokeRustBackend('archiveMessage', { accountId, messageId }),
  restoreMessage: (accountId, messageId) => invokeRustBackend('restoreMessage', { accountId, messageId }),
  moveMessage: (accountId, messageId, folderId) => invokeRustBackend('moveMessage', { accountId, messageId, folderId }),
  batchMoveMessages: (accountId, messageIds, folderId) => invokeRustBackend('batchMoveMessages', { accountId, messageIds, folderId }),
  markAllRead: (accountId, folderId) => invokeRustBackend('markAllRead', { accountId, folderId }),
  deleteMessage: (accountId, messageId) => invokeRustBackend('deleteMessage', { accountId, messageId }),
  batchDeleteMessages: (accountId, messageIds) => invokeRustBackend('batchDeleteMessages', { accountId, messageIds }),
  deleteAccount: (accountId) => invokeRustBackend('deleteAccount', { accountId }),
  syncAccount: syncAccountDeduped,
  getMailboxBundle: (accountId) => invokeRustBackend('getMailboxBundle', { accountId }),
  getAccountUnreadSnapshot: (accountId) => invokeRustBackend('getAccountUnreadSnapshot', { accountId }),
  getAttachmentContent: (accountId, messageId, attachmentId) => invokeRustBackend('getAttachmentContent', { accountId, messageId, attachmentId }),
  getAccountConfig: (accountId) => invokeRustBackend('getAccountConfig', { accountId }),
  updateAccount: (accountId, draft) => invokeRustBackend('updateAccount', { accountId, draft }),
  getAccountQuota: (accountId) => invokeRustBackend('getAccountQuota', { accountId }),
  listOAuthProviders: () => invokeRustBackend('listOAuthProviders'),
  authorizeOAuth,
  handleOAuthCallbackUrl: async (rawUrl) => handleOAuthCallbackUrl(rawUrl),
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
