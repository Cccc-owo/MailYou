import type { AccountSetupDraft, MailAccount, OAuthProviderAvailability } from '@/types/account'
import type { Contact, ContactGroup } from '@/types/contact'
import type {
  AttachmentContent,
  DraftMessage,
  MailMessage,
  MailboxBundle,
  MailboxFolder,
  SyncStatus,
} from '@/types/mail'
import type { StorageSecurityStatus } from '@/types/security'

export interface RustBackendMethodMap {
  healthCheck: {
    params: undefined
    result: { ok: true; backend: 'rust'; version: string }
  }
  listAccounts: {
    params: undefined
    result: MailAccount[]
  }
  createAccount: {
    params: AccountSetupDraft
    result: MailAccount
  }
  testAccountConnection: {
    params: AccountSetupDraft
    result: SyncStatus
  }
  listFolders: {
    params: { accountId: string }
    result: MailboxFolder[]
  }
  listMessages: {
    params: { accountId: string; folderId: string }
    result: MailMessage[]
  }
  searchMessages: {
    params: { accountId: string; query: string }
    result: MailMessage[]
  }
  getMessage: {
    params: { accountId: string; messageId: string }
    result: MailMessage | null
  }
  saveDraft: {
    params: DraftMessage
    result: DraftMessage
  }
  sendMessage: {
    params: DraftMessage
    result: { ok: true; queuedAt: string }
  }
  toggleStar: {
    params: { accountId: string; messageId: string }
    result: MailMessage | null
  }
  toggleRead: {
    params: { accountId: string; messageId: string }
    result: MailMessage | null
  }
  archiveMessage: {
    params: { accountId: string; messageId: string }
    result: MailMessage | null
  }
  restoreMessage: {
    params: { accountId: string; messageId: string }
    result: MailMessage | null
  }
  moveMessage: {
    params: { accountId: string; messageId: string; folderId: string }
    result: MailMessage | null
  }
  markAllRead: {
    params: { accountId: string; folderId: string }
    result: void
  }
  deleteMessage: {
    params: { accountId: string; messageId: string }
    result: void
  }
  deleteAccount: {
    params: { accountId: string }
    result: void
  }
  syncAccount: {
    params: { accountId: string }
    result: SyncStatus
  }
  getMailboxBundle: {
    params: { accountId: string }
    result: MailboxBundle
  }
  getAttachmentContent: {
    params: { accountId: string; messageId: string; attachmentId: string }
    result: AttachmentContent
  }
  getAccountConfig: {
    params: { accountId: string }
    result: AccountSetupDraft
  }
  updateAccount: {
    params: { accountId: string; draft: AccountSetupDraft }
    result: MailAccount
  }
  listOAuthProviders: {
    params: undefined
    result: OAuthProviderAvailability[]
  }
  listContacts: {
    params: { groupId?: string }
    result: Contact[]
  }
  createContact: {
    params: Contact
    result: Contact
  }
  updateContact: {
    params: { contactId: string; contact: Contact }
    result: Contact
  }
  deleteContact: {
    params: { contactId: string }
    result: void
  }
  searchContacts: {
    params: { query: string }
    result: Contact[]
  }
  listContactGroups: {
    params: undefined
    result: ContactGroup[]
  }
  createContactGroup: {
    params: { name: string }
    result: ContactGroup
  }
  updateContactGroup: {
    params: { groupId: string; name: string }
    result: ContactGroup
  }
  deleteContactGroup: {
    params: { groupId: string }
    result: void
  }
  uploadContactAvatar: {
    params: { contactId: string; dataBase64: string; mimeType: string }
    result: Contact
  }
  deleteContactAvatar: {
    params: { contactId: string }
    result: Contact
  }
  getContactAvatar: {
    params: { contactId: string }
    result: AttachmentContent | null
  }
  getSecurityStatus: {
    params: undefined
    result: StorageSecurityStatus
  }
  unlockStorage: {
    params: { password: string }
    result: StorageSecurityStatus
  }
  setMasterPassword: {
    params: { currentPassword: string | null; newPassword: string }
    result: StorageSecurityStatus
  }
  clearMasterPassword: {
    params: { currentPassword: string }
    result: StorageSecurityStatus
  }
  getStorageDir: {
    params: undefined
    result: string
  }
}

export type RustBackendMethod = keyof RustBackendMethodMap

export type RustBackendRequest<M extends RustBackendMethod = RustBackendMethod> =
  RustBackendMethodMap[M]['params'] extends undefined
    ? { id: number; method: M }
    : { id: number; method: M; params: RustBackendMethodMap[M]['params'] }

export interface RustBackendError {
  code: string
  message: string
}

export type RustBackendResponse<M extends RustBackendMethod = RustBackendMethod> =
  | {
      id: number
      ok: true
      result: RustBackendMethodMap[M]['result']
    }
  | {
      id: number
      ok: false
      error: RustBackendError
    }

export interface RustBackendMailboxChangedEvent {
  event: 'mailboxChanged'
  payload: {
    accountId: string
    source: 'idle' | 'sync'
  }
}

export type RustBackendEvent = RustBackendMailboxChangedEvent

export type RustBackendMessage<M extends RustBackendMethod = RustBackendMethod> =
  | RustBackendResponse<M>
  | RustBackendEvent
