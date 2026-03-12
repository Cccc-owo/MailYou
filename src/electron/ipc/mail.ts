import { ipcMain } from 'electron'
import { mailBackend } from '../backend/mailBackend'

let registered = false

export const registerMailIpc = () => {
  if (registered) {
    return
  }

  registered = true

  ipcMain.handle('mail:listAccounts', () => mailBackend.listAccounts())
  ipcMain.handle('mail:createAccount', (_event, draft) => mailBackend.createAccount(draft))
  ipcMain.handle('mail:testAccountConnection', (_event, draft) => mailBackend.testAccountConnection(draft))
  ipcMain.handle('mail:listFolders', (_event, accountId) => mailBackend.listFolders(accountId))
  ipcMain.handle('mail:listMessages', (_event, accountId, folderId) =>
    mailBackend.listMessages(accountId, folderId),
  )
  ipcMain.handle('mail:getMessage', (_event, accountId, messageId) =>
    mailBackend.getMessage(accountId, messageId),
  )
  ipcMain.handle('mail:saveDraft', (_event, draft) => mailBackend.saveDraft(draft))
  ipcMain.handle('mail:sendMessage', (_event, draft) => mailBackend.sendMessage(draft))
  ipcMain.handle('mail:toggleStar', (_event, accountId, messageId) =>
    mailBackend.toggleStar(accountId, messageId),
  )
  ipcMain.handle('mail:toggleRead', (_event, accountId, messageId) =>
    mailBackend.toggleRead(accountId, messageId),
  )
  ipcMain.handle('mail:archiveMessage', (_event, accountId, messageId) =>
    mailBackend.archiveMessage(accountId, messageId),
  )
  ipcMain.handle('mail:restoreMessage', (_event, accountId, messageId) =>
    mailBackend.restoreMessage(accountId, messageId),
  )
  ipcMain.handle('mail:moveMessage', (_event, accountId, messageId, folderId) =>
    mailBackend.moveMessage(accountId, messageId, folderId),
  )
  ipcMain.handle('mail:deleteMessage', (_event, accountId, messageId) =>
    mailBackend.deleteMessage(accountId, messageId),
  )
  ipcMain.handle('mail:syncAccount', (_event, accountId) => mailBackend.syncAccount(accountId))
  ipcMain.handle('mail:getMailboxBundle', (_event, accountId) => mailBackend.getMailboxBundle(accountId))
}
