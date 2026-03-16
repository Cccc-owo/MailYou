import type { MailRepository } from '@/shared/mail/mailRepository'
import type { MailyouBridge } from '@/shared/mail/bridge'

const getBridge = (): MailyouBridge => {
  if (!window.mailyou) {
    throw new Error('Mailyou bridge is not available. Use the Electron desktop target or the explicit web mock entrypoint.')
  }

  return window.mailyou
}

export const electronMailRepository: MailRepository = {
  listAccounts: () => getBridge().listAccounts(),
  createAccount: (draft) => getBridge().createAccount(draft),
  testAccountConnection: (draft) => getBridge().testAccountConnection(draft),
  listFolders: (accountId) => getBridge().listFolders(accountId),
  listMessages: (accountId, folderId) => getBridge().listMessages(accountId, folderId),
  searchMessages: (accountId, query) => getBridge().searchMessages(accountId, query),
  getMessage: (accountId, messageId) => getBridge().getMessage(accountId, messageId),
  saveDraft: (draft) => getBridge().saveDraft(draft),
  sendMessage: (draft) => getBridge().sendMessage(draft),
  toggleStar: (accountId, messageId) => getBridge().toggleStar(accountId, messageId),
  toggleRead: (accountId, messageId) => getBridge().toggleRead(accountId, messageId),
  archiveMessage: (accountId, messageId) => getBridge().archiveMessage(accountId, messageId),
  restoreMessage: (accountId, messageId) => getBridge().restoreMessage(accountId, messageId),
  moveMessage: (accountId, messageId, folderId) => getBridge().moveMessage(accountId, messageId, folderId),
  markAllRead: (accountId, folderId) => getBridge().markAllRead(accountId, folderId),
  deleteMessage: (accountId, messageId) => getBridge().deleteMessage(accountId, messageId),
  deleteAccount: (accountId) => getBridge().deleteAccount(accountId),
  syncAccount: (accountId) => getBridge().syncAccount(accountId),
  getMailboxBundle: (accountId) => getBridge().getMailboxBundle(accountId),
  getAttachmentContent: (accountId, messageId, attachmentId) => getBridge().getAttachmentContent(accountId, messageId, attachmentId),
  getAccountConfig: (accountId) => getBridge().getAccountConfig(accountId),
  updateAccount: (accountId, draft) => getBridge().updateAccount(accountId, draft),
  listOAuthProviders: () => getBridge().listOAuthProviders(),
  authorizeOAuth: (request) => getBridge().authorizeOAuth(request),
  listContacts: (groupId) => getBridge().listContacts(groupId),
  createContact: (contact) => getBridge().createContact(contact),
  updateContact: (contactId, contact) => getBridge().updateContact(contactId, contact),
  deleteContact: (contactId) => getBridge().deleteContact(contactId),
  searchContacts: (query) => getBridge().searchContacts(query),
  listContactGroups: () => getBridge().listContactGroups(),
  createContactGroup: (name) => getBridge().createContactGroup(name),
  updateContactGroup: (groupId, name) => getBridge().updateContactGroup(groupId, name),
  deleteContactGroup: (groupId) => getBridge().deleteContactGroup(groupId),
  uploadContactAvatar: (contactId, dataBase64, mimeType) => getBridge().uploadContactAvatar(contactId, dataBase64, mimeType),
  deleteContactAvatar: (contactId) => getBridge().deleteContactAvatar(contactId),
  getContactAvatar: (contactId) => getBridge().getContactAvatar(contactId),
  getSecurityStatus: () => getBridge().getSecurityStatus(),
  unlockStorage: (password) => getBridge().unlockStorage(password),
  setMasterPassword: (currentPassword, newPassword) => getBridge().setMasterPassword(currentPassword, newPassword),
  clearMasterPassword: (currentPassword) => getBridge().clearMasterPassword(currentPassword),
  lockCurrentSession: () => getBridge().lockCurrentSession(),
  getStorageDir: () => getBridge().getStorageDir(),
}
