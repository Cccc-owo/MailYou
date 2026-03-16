import { contextBridge, ipcRenderer } from 'electron'
import type { MailyouBridge } from '../shared/mail/bridge'
import type { WindowControlsBridge } from '../shared/window/bridge'

const mailyou: MailyouBridge = {
  listAccounts: () => ipcRenderer.invoke('mail:listAccounts'),
  createAccount: (draft) => ipcRenderer.invoke('mail:createAccount', draft),
  testAccountConnection: (draft) => ipcRenderer.invoke('mail:testAccountConnection', draft),
  listFolders: (accountId) => ipcRenderer.invoke('mail:listFolders', accountId),
  listMessages: (accountId, folderId) => ipcRenderer.invoke('mail:listMessages', accountId, folderId),
  searchMessages: (accountId, query) => ipcRenderer.invoke('mail:searchMessages', accountId, query),
  getMessage: (accountId, messageId) => ipcRenderer.invoke('mail:getMessage', accountId, messageId),
  saveDraft: (draft) => ipcRenderer.invoke('mail:saveDraft', draft),
  sendMessage: (draft) => ipcRenderer.invoke('mail:sendMessage', draft),
  toggleStar: (accountId, messageId) => ipcRenderer.invoke('mail:toggleStar', accountId, messageId),
  toggleRead: (accountId, messageId) => ipcRenderer.invoke('mail:toggleRead', accountId, messageId),
  archiveMessage: (accountId, messageId) => ipcRenderer.invoke('mail:archiveMessage', accountId, messageId),
  restoreMessage: (accountId, messageId) => ipcRenderer.invoke('mail:restoreMessage', accountId, messageId),
  moveMessage: (accountId, messageId, folderId) => ipcRenderer.invoke('mail:moveMessage', accountId, messageId, folderId),
  markAllRead: (accountId, folderId) => ipcRenderer.invoke('mail:markAllRead', accountId, folderId),
  deleteMessage: (accountId, messageId) => ipcRenderer.invoke('mail:deleteMessage', accountId, messageId),
  deleteAccount: (accountId) => ipcRenderer.invoke('mail:deleteAccount', accountId),
  syncAccount: (accountId) => ipcRenderer.invoke('mail:syncAccount', accountId),
  getMailboxBundle: (accountId) => ipcRenderer.invoke('mail:getMailboxBundle', accountId),
  getAttachmentContent: (accountId, messageId, attachmentId) => ipcRenderer.invoke('mail:getAttachmentContent', accountId, messageId, attachmentId),
  getAccountConfig: (accountId) => ipcRenderer.invoke('mail:getAccountConfig', accountId),
  updateAccount: (accountId, draft) => ipcRenderer.invoke('mail:updateAccount', accountId, draft),
  listOAuthProviders: () => ipcRenderer.invoke('mail:listOAuthProviders'),
  authorizeOAuth: (request) => ipcRenderer.invoke('mail:authorizeOAuth', request),
  listContacts: (groupId) => ipcRenderer.invoke('mail:listContacts', groupId),
  createContact: (contact) => ipcRenderer.invoke('mail:createContact', contact),
  updateContact: (contactId, contact) => ipcRenderer.invoke('mail:updateContact', contactId, contact),
  deleteContact: (contactId) => ipcRenderer.invoke('mail:deleteContact', contactId),
  searchContacts: (query) => ipcRenderer.invoke('mail:searchContacts', query),
  listContactGroups: () => ipcRenderer.invoke('mail:listContactGroups'),
  createContactGroup: (name) => ipcRenderer.invoke('mail:createContactGroup', name),
  updateContactGroup: (groupId, name) => ipcRenderer.invoke('mail:updateContactGroup', groupId, name),
  deleteContactGroup: (groupId) => ipcRenderer.invoke('mail:deleteContactGroup', groupId),
  uploadContactAvatar: (contactId, dataBase64, mimeType) => ipcRenderer.invoke('mail:uploadContactAvatar', contactId, dataBase64, mimeType),
  deleteContactAvatar: (contactId) => ipcRenderer.invoke('mail:deleteContactAvatar', contactId),
  getContactAvatar: (contactId) => ipcRenderer.invoke('mail:getContactAvatar', contactId),
  getSecurityStatus: () => ipcRenderer.invoke('mail:getSecurityStatus'),
  unlockStorage: (password) => ipcRenderer.invoke('mail:unlockStorage', password),
  setMasterPassword: (currentPassword, newPassword) => ipcRenderer.invoke('mail:setMasterPassword', currentPassword, newPassword),
  clearMasterPassword: (currentPassword) => ipcRenderer.invoke('mail:clearMasterPassword', currentPassword),
  lockCurrentSession: () => ipcRenderer.invoke('mail:lockCurrentSession'),
  getStorageDir: () => ipcRenderer.invoke('mail:getStorageDir'),
  onBackgroundSync: (callback) => {
    const listener = (_event: Electron.IpcRendererEvent, accountId: string) => callback(accountId)
    ipcRenderer.on('mail:backgroundSyncComplete', listener)
    return () => {
      ipcRenderer.removeListener('mail:backgroundSyncComplete', listener)
    }
  },
}

const windowControls: WindowControlsBridge = {
  minimize: () => ipcRenderer.invoke('window:minimize'),
  toggleMaximize: () => ipcRenderer.invoke('window:toggleMaximize'),
  close: () => ipcRenderer.invoke('window:close'),
  isMaximized: () => ipcRenderer.invoke('window:isMaximized'),
  openExternal: (url) => ipcRenderer.invoke('window:openExternal', url),
  focus: () => ipcRenderer.invoke('window:focus'),
  setBackgroundSyncInterval: (minutes) => ipcRenderer.invoke('window:setBackgroundSyncInterval', minutes),
  exportPdf: (html, fileName) => ipcRenderer.invoke('window:exportPdf', html, fileName),
  openTextFile: (filters) => ipcRenderer.invoke('window:openTextFile', filters),
  saveTextFile: (content, suggestedName, filters) =>
    ipcRenderer.invoke('window:saveTextFile', content, suggestedName, filters),
}

contextBridge.exposeInMainWorld('mailyou', mailyou)
contextBridge.exposeInMainWorld('windowControls', windowControls)
