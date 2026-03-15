import type { AccountSetupDraft, MailAccount } from '@/types/account'
import type { Contact, ContactGroup } from '@/types/contact'
import type {
  AttachmentContent,
  DraftMessage,
  MailMessage,
  MailboxBundle,
  MailboxFolder,
  SyncStatus,
} from '@/types/mail'

export interface MailRepository {
  listAccounts(): Promise<MailAccount[]>
  createAccount(draft: AccountSetupDraft): Promise<MailAccount>
  testAccountConnection(draft: AccountSetupDraft): Promise<SyncStatus>
  listFolders(accountId: string): Promise<MailboxFolder[]>
  listMessages(accountId: string, folderId: string): Promise<MailMessage[]>
  getMessage(accountId: string, messageId: string): Promise<MailMessage | null>
  saveDraft(draft: DraftMessage): Promise<DraftMessage>
  sendMessage(draft: DraftMessage): Promise<{ ok: true; queuedAt: string }>
  toggleStar(accountId: string, messageId: string): Promise<MailMessage | null>
  toggleRead(accountId: string, messageId: string): Promise<MailMessage | null>
  archiveMessage(accountId: string, messageId: string): Promise<MailMessage | null>
  restoreMessage(accountId: string, messageId: string): Promise<MailMessage | null>
  moveMessage(accountId: string, messageId: string, folderId: string): Promise<MailMessage | null>
  markAllRead(accountId: string, folderId: string): Promise<void>
  deleteMessage(accountId: string, messageId: string): Promise<void>
  deleteAccount(accountId: string): Promise<void>
  syncAccount(accountId: string): Promise<SyncStatus>
  getMailboxBundle(accountId: string): Promise<MailboxBundle>
  getAttachmentContent(accountId: string, messageId: string, attachmentId: string): Promise<AttachmentContent>
  getAccountConfig(accountId: string): Promise<AccountSetupDraft>
  updateAccount(accountId: string, draft: AccountSetupDraft): Promise<MailAccount>
  listContacts(groupId?: string): Promise<Contact[]>
  createContact(contact: Contact): Promise<Contact>
  updateContact(contactId: string, contact: Contact): Promise<Contact>
  deleteContact(contactId: string): Promise<void>
  searchContacts(query: string): Promise<Contact[]>
  listContactGroups(): Promise<ContactGroup[]>
  createContactGroup(name: string): Promise<ContactGroup>
  updateContactGroup(groupId: string, name: string): Promise<ContactGroup>
  deleteContactGroup(groupId: string): Promise<void>
  uploadContactAvatar(contactId: string, dataBase64: string, mimeType: string): Promise<Contact>
  deleteContactAvatar(contactId: string): Promise<Contact>
  getStorageDir(): Promise<string>
}
