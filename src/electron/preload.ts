import { contextBridge, ipcRenderer } from 'electron'
import type { MailstackBridge } from '../shared/mail/bridge'
import type { WindowControlsBridge } from '../shared/window/bridge'

const mailstack: MailstackBridge = {
  listAccounts: () => ipcRenderer.invoke('mail:listAccounts'),
  createAccount: (draft) => ipcRenderer.invoke('mail:createAccount', draft),
  listFolders: (accountId) => ipcRenderer.invoke('mail:listFolders', accountId),
  listMessages: (accountId, folderId) => ipcRenderer.invoke('mail:listMessages', accountId, folderId),
  getMessage: (accountId, messageId) => ipcRenderer.invoke('mail:getMessage', accountId, messageId),
  saveDraft: (draft) => ipcRenderer.invoke('mail:saveDraft', draft),
  sendMessage: (draft) => ipcRenderer.invoke('mail:sendMessage', draft),
  toggleStar: (accountId, messageId) => ipcRenderer.invoke('mail:toggleStar', accountId, messageId),
  toggleRead: (accountId, messageId) => ipcRenderer.invoke('mail:toggleRead', accountId, messageId),
  deleteMessage: (accountId, messageId) => ipcRenderer.invoke('mail:deleteMessage', accountId, messageId),
  syncAccount: (accountId) => ipcRenderer.invoke('mail:syncAccount', accountId),
  getMailboxBundle: (accountId) => ipcRenderer.invoke('mail:getMailboxBundle', accountId),
}

const windowControls: WindowControlsBridge = {
  minimize: () => ipcRenderer.invoke('window:minimize'),
  toggleMaximize: () => ipcRenderer.invoke('window:toggleMaximize'),
  close: () => ipcRenderer.invoke('window:close'),
  isMaximized: () => ipcRenderer.invoke('window:isMaximized'),
}

contextBridge.exposeInMainWorld('mailstack', mailstack)
contextBridge.exposeInMainWorld('windowControls', windowControls)
