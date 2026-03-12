import type { AccountSetupDraft, MailAccount } from '@/types/account'
import type {
  DraftMessage,
  MailMessage,
  MailboxBundle,
  MailboxFolder,
  SyncStatus,
} from '@/types/mail'

export interface MailRepository {
  listAccounts(): Promise<MailAccount[]>
  createAccount(draft: AccountSetupDraft): Promise<MailAccount>
  listFolders(accountId: string): Promise<MailboxFolder[]>
  listMessages(accountId: string, folderId: string): Promise<MailMessage[]>
  getMessage(accountId: string, messageId: string): Promise<MailMessage | null>
  saveDraft(draft: DraftMessage): Promise<DraftMessage>
  sendMessage(draft: DraftMessage): Promise<{ ok: true; queuedAt: string }>
  toggleStar(accountId: string, messageId: string): Promise<MailMessage | null>
  toggleRead(accountId: string, messageId: string): Promise<MailMessage | null>
  deleteMessage(accountId: string, messageId: string): Promise<void>
  syncAccount(accountId: string): Promise<SyncStatus>
  getMailboxBundle(accountId: string): Promise<MailboxBundle>
}
