import type { AccountSetupDraft, MailAccount } from '@/types/account'
import type {
  DraftMessage,
  MailMessage,
  MailboxBundle,
  MailboxFolder,
  SyncStatus,
} from '@/types/mail'

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
