import type {
  AccountSetupDraft,
  MailAccount,
  OAuthAuthorizationRequest,
  OAuthAuthorizationResult,
  OAuthProviderAvailability,
} from '@/types/account'
import type { Contact, ContactGroup } from '@/types/contact'
import type {
  MailLabel,
  AttachmentContent,
  DraftMessage,
  MailMessage,
  MailboxBundle,
  MailboxFolder,
  SyncStatus,
} from '@/types/mail'
import type { StorageSecurityStatus } from '@/types/security'

export interface MailRepository {
  listAccounts(): Promise<MailAccount[]>
  createAccount(draft: AccountSetupDraft): Promise<MailAccount>
  testAccountConnection(draft: AccountSetupDraft): Promise<SyncStatus>
  listFolders(accountId: string): Promise<MailboxFolder[]>
  createFolder(accountId: string, name: string): Promise<MailboxFolder[]>
  renameFolder(accountId: string, folderId: string, name: string): Promise<MailboxFolder[]>
  deleteFolder(accountId: string, folderId: string): Promise<MailboxFolder[]>
  listMessages(accountId: string, folderId: string): Promise<MailMessage[]>
  getDraft(accountId: string, draftId: string): Promise<DraftMessage | null>
  searchMessages(accountId: string, query: string): Promise<MailMessage[]>
  listLabels(accountId: string): Promise<MailLabel[]>
  getMessage(accountId: string, messageId: string): Promise<MailMessage | null>
  addLabel(accountId: string, messageId: string, label: string): Promise<MailMessage | null>
  removeLabel(accountId: string, messageId: string, label: string): Promise<MailMessage | null>
  renameLabel(accountId: string, label: string, newLabel: string): Promise<MailLabel[]>
  deleteLabel(accountId: string, label: string): Promise<MailLabel[]>
  saveDraft(draft: DraftMessage): Promise<DraftMessage>
  sendMessage(draft: DraftMessage): Promise<{ ok: true; queuedAt: string }>
  toggleStar(accountId: string, messageId: string): Promise<MailMessage | null>
  toggleRead(accountId: string, messageId: string): Promise<MailMessage | null>
  batchToggleRead(accountId: string, messageIds: string[], isRead: boolean): Promise<void>
  archiveMessage(accountId: string, messageId: string): Promise<MailMessage | null>
  restoreMessage(accountId: string, messageId: string): Promise<MailMessage | null>
  moveMessage(accountId: string, messageId: string, folderId: string): Promise<MailMessage | null>
  batchMoveMessages(accountId: string, messageIds: string[], folderId: string): Promise<void>
  markAllRead(accountId: string, folderId: string): Promise<void>
  deleteMessage(accountId: string, messageId: string): Promise<void>
  batchDeleteMessages(accountId: string, messageIds: string[]): Promise<void>
  deleteAccount(accountId: string): Promise<void>
  syncAccount(accountId: string): Promise<SyncStatus>
  getMailboxBundle(accountId: string): Promise<MailboxBundle>
  getAttachmentContent(accountId: string, messageId: string, attachmentId: string): Promise<AttachmentContent>
  getAccountConfig(accountId: string): Promise<AccountSetupDraft>
  updateAccount(accountId: string, draft: AccountSetupDraft): Promise<MailAccount>
  listOAuthProviders(): Promise<OAuthProviderAvailability[]>
  authorizeOAuth(request: OAuthAuthorizationRequest): Promise<OAuthAuthorizationResult>
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
  getContactAvatar(contactId: string): Promise<AttachmentContent | null>
  getSecurityStatus(): Promise<StorageSecurityStatus>
  unlockStorage(password: string): Promise<StorageSecurityStatus>
  setMasterPassword(currentPassword: string | null, newPassword: string): Promise<StorageSecurityStatus>
  clearMasterPassword(currentPassword: string): Promise<StorageSecurityStatus>
  lockCurrentSession(): Promise<StorageSecurityStatus>
  getStorageDir(): Promise<string>
}
