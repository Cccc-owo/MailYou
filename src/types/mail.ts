export type MailFolderKind =
  | 'inbox'
  | 'sent'
  | 'drafts'
  | 'trash'
  | 'starred'
  | 'archive'
  | 'custom'

export interface MailboxFolder {
  id: string
  accountId: string
  name: string
  kind: MailFolderKind
  unreadCount: number
  totalCount: number
  icon: string
  imapName?: string
}

export interface AttachmentMeta {
  id: string
  fileName: string
  mimeType: string
  sizeBytes: number
}

export interface MailMessage {
  id: string
  accountId: string
  folderId: string
  threadId: string
  subject: string
  preview: string
  body: string
  from: string
  fromEmail: string
  to: string[]
  cc: string[]
  sentAt: string
  receivedAt: string
  isRead: boolean
  isStarred: boolean
  hasAttachments: boolean
  attachments: AttachmentMeta[]
  labels: string[]
  imapUid?: number
}

export interface MailThread {
  id: string
  accountId: string
  subject: string
  messageIds: string[]
  lastMessageAt: string
  unreadCount: number
}

export interface MailThreadSummary {
  threadId: string
  accountId: string
  message: MailMessage
  messageCount: number
  unreadCount: number
  participants: string[]
}

export interface AttachmentContent {
  fileName: string
  mimeType: string
  dataBase64: string
}

export interface DraftAttachment {
  fileName: string
  mimeType: string
  dataBase64: string
}

export interface DraftMessage {
  id: string
  accountId: string
  to: string
  cc: string
  bcc: string
  subject: string
  body: string
  inReplyToMessageId?: string
  forwardFromMessageId?: string
  attachments: DraftAttachment[]
}

export interface SyncStatus {
  accountId: string
  state: 'idle' | 'syncing' | 'error'
  message: string
  updatedAt: string
}

export interface MailboxBundle {
  accountId: string
  folders: MailboxFolder[]
  messages: MailMessage[]
  threads: MailThread[]
  syncStatus: SyncStatus
}
