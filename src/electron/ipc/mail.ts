import { ipcMain } from 'electron'
import type { AccountSetupDraft } from '@/types/account'
import type { DraftMessage } from '@/types/mail'
import type { Contact } from '@/types/contact'
import { mailBackend } from '../backend/mailBackend'

let registered = false

type IpcArgs = unknown[]

const handle = (channel: string, fn: (...args: IpcArgs) => Promise<unknown>) => {
  ipcMain.handle(channel, async (_event, ...args: IpcArgs) => {
    const tag = channel.replace('mail:', '')
    const argSummary = args
      .map((a) => (typeof a === 'string' ? a : typeof a === 'object' ? '{...}' : String(a)))
      .join(', ')
    console.log(`[ipc] ${tag}(${argSummary})`)

    const start = Date.now()
    try {
      const result = await fn(...args)
      console.log(`[ipc] ${tag} → ok (${Date.now() - start}ms)`)
      return result
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err)
      console.error(`[ipc] ${tag} → error (${Date.now() - start}ms): ${msg}`)
      throw err
    }
  })
}

export const registerMailIpc = () => {
  if (registered) {
    return
  }

  registered = true

  handle('mail:listAccounts', () => mailBackend.listAccounts())
  handle('mail:createAccount', (draft) => mailBackend.createAccount(draft as AccountSetupDraft))
  handle('mail:testAccountConnection', (draft) =>
    mailBackend.testAccountConnection(draft as AccountSetupDraft),
  )
  handle('mail:listFolders', (accountId) => mailBackend.listFolders(accountId as string))
  handle('mail:listMessages', (accountId, folderId) =>
    mailBackend.listMessages(accountId as string, folderId as string),
  )
  handle('mail:getMessage', (accountId, messageId) =>
    mailBackend.getMessage(accountId as string, messageId as string),
  )
  handle('mail:saveDraft', (draft) => mailBackend.saveDraft(draft as DraftMessage))
  handle('mail:sendMessage', (draft) => mailBackend.sendMessage(draft as DraftMessage))
  handle('mail:toggleStar', (accountId, messageId) =>
    mailBackend.toggleStar(accountId as string, messageId as string),
  )
  handle('mail:toggleRead', (accountId, messageId) =>
    mailBackend.toggleRead(accountId as string, messageId as string),
  )
  handle('mail:archiveMessage', (accountId, messageId) =>
    mailBackend.archiveMessage(accountId as string, messageId as string),
  )
  handle('mail:restoreMessage', (accountId, messageId) =>
    mailBackend.restoreMessage(accountId as string, messageId as string),
  )
  handle('mail:moveMessage', (accountId, messageId, folderId) =>
    mailBackend.moveMessage(accountId as string, messageId as string, folderId as string),
  )
  handle('mail:markAllRead', (accountId, folderId) =>
    mailBackend.markAllRead(accountId as string, folderId as string),
  )
  handle('mail:deleteMessage', (accountId, messageId) =>
    mailBackend.deleteMessage(accountId as string, messageId as string),
  )
  handle('mail:deleteAccount', (accountId) => mailBackend.deleteAccount(accountId as string))
  handle('mail:syncAccount', (accountId) => mailBackend.syncAccount(accountId as string))
  handle('mail:getMailboxBundle', (accountId) =>
    mailBackend.getMailboxBundle(accountId as string),
  )
  handle('mail:getAttachmentContent', (accountId, messageId, attachmentId) =>
    mailBackend.getAttachmentContent(accountId as string, messageId as string, attachmentId as string),
  )
  handle('mail:getAccountConfig', (accountId) =>
    mailBackend.getAccountConfig(accountId as string),
  )
  handle('mail:updateAccount', (accountId, draft) =>
    mailBackend.updateAccount(accountId as string, draft as AccountSetupDraft),
  )
  handle('mail:listOAuthProviders', () => mailBackend.listOAuthProviders())
  handle('mail:authorizeOAuth', (request) => mailBackend.authorizeOAuth(request as never))
  handle('mail:listContacts', (groupId) => mailBackend.listContacts(groupId as string | undefined))
  handle('mail:createContact', (contact) => mailBackend.createContact(contact as Contact))
  handle('mail:updateContact', (contactId, contact) =>
    mailBackend.updateContact(contactId as string, contact as Contact),
  )
  handle('mail:deleteContact', (contactId) => mailBackend.deleteContact(contactId as string))
  handle('mail:searchContacts', (query) => mailBackend.searchContacts(query as string))
  handle('mail:listContactGroups', () => mailBackend.listContactGroups())
  handle('mail:createContactGroup', (name) => mailBackend.createContactGroup(name as string))
  handle('mail:updateContactGroup', (groupId, name) =>
    mailBackend.updateContactGroup(groupId as string, name as string),
  )
  handle('mail:deleteContactGroup', (groupId) => mailBackend.deleteContactGroup(groupId as string))
  handle('mail:uploadContactAvatar', (contactId, dataBase64, mimeType) =>
    mailBackend.uploadContactAvatar(contactId as string, dataBase64 as string, mimeType as string),
  )
  handle('mail:deleteContactAvatar', (contactId) => mailBackend.deleteContactAvatar(contactId as string))
  handle('mail:getStorageDir', () => mailBackend.getStorageDir())
}
