import { contextBridge, ipcRenderer } from 'electron'
import type { MailyouBridge } from '../shared/mail/bridge'
import type { WindowControlsBridge } from '../shared/window/bridge'

const mailyou: MailyouBridge = {
  listAccounts: () => ipcRenderer.invoke('mail:listAccounts'),
  createAccount: (draft) => ipcRenderer.invoke('mail:createAccount', draft),
  testAccountConnection: (draft) => ipcRenderer.invoke('mail:testAccountConnection', draft),
  listFolders: (accountId) => ipcRenderer.invoke('mail:listFolders', accountId),
  listMessages: (accountId, folderId) => ipcRenderer.invoke('mail:listMessages', accountId, folderId),
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
}

const windowControls: WindowControlsBridge = {
  minimize: () => ipcRenderer.invoke('window:minimize'),
  toggleMaximize: () => ipcRenderer.invoke('window:toggleMaximize'),
  close: () => ipcRenderer.invoke('window:close'),
  isMaximized: () => ipcRenderer.invoke('window:isMaximized'),
  openExternal: (url) => ipcRenderer.invoke('window:openExternal', url),
}

contextBridge.exposeInMainWorld('mailyou', mailyou)
contextBridge.exposeInMainWorld('windowControls', windowControls)
