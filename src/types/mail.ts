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
}

export interface MailThread {
  id: string
  accountId: string
  subject: string
  messageIds: string[]
  lastMessageAt: string
  unreadCount: number
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
