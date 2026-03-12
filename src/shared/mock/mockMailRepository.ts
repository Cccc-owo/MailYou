import type { AccountSetupDraft, MailAccount } from '@/types/account'
import type {
  DraftMessage,
  MailMessage,
  MailboxBundle,
  MailThread,
  MailboxFolder,
  SyncStatus,
} from '@/types/mail'
import type { MailRepository } from '@/shared/mail/mailRepository'

const now = new Date('2026-03-12T09:45:00.000Z')

const accounts: MailAccount[] = [
  {
    id: 'acc-work',
    name: 'MailStack Work',
    email: 'hello@mailstack.dev',
    provider: 'Fastmail',
    color: '#6D5DFB',
    initials: 'MW',
    unreadCount: 6,
    status: 'syncing',
    lastSyncedAt: new Date(now.getTime() - 1000 * 60 * 4).toISOString(),
  },
  {
    id: 'acc-personal',
    name: 'Personal',
    email: 'iscccc@example.com',
    provider: 'Gmail',
    color: '#0F9D58',
    initials: 'IP',
    unreadCount: 3,
    status: 'connected',
    lastSyncedAt: new Date(now.getTime() - 1000 * 60 * 18).toISOString(),
  },
]

const folders: MailboxFolder[] = [
  { id: 'inbox-work', accountId: 'acc-work', name: 'Inbox', kind: 'inbox', unreadCount: 4, totalCount: 24, icon: 'mdi-inbox-arrow-down' },
  { id: 'starred-work', accountId: 'acc-work', name: 'Starred', kind: 'starred', unreadCount: 1, totalCount: 5, icon: 'mdi-star-outline' },
  { id: 'sent-work', accountId: 'acc-work', name: 'Sent', kind: 'sent', unreadCount: 0, totalCount: 12, icon: 'mdi-send-outline' },
  { id: 'drafts-work', accountId: 'acc-work', name: 'Drafts', kind: 'drafts', unreadCount: 0, totalCount: 2, icon: 'mdi-file-document-edit-outline' },
  { id: 'trash-work', accountId: 'acc-work', name: 'Trash', kind: 'trash', unreadCount: 0, totalCount: 1, icon: 'mdi-delete-outline' },
  { id: 'inbox-personal', accountId: 'acc-personal', name: 'Inbox', kind: 'inbox', unreadCount: 3, totalCount: 15, icon: 'mdi-inbox-arrow-down' },
  { id: 'starred-personal', accountId: 'acc-personal', name: 'Starred', kind: 'starred', unreadCount: 1, totalCount: 3, icon: 'mdi-star-outline' },
  { id: 'sent-personal', accountId: 'acc-personal', name: 'Sent', kind: 'sent', unreadCount: 0, totalCount: 8, icon: 'mdi-send-outline' },
  { id: 'drafts-personal', accountId: 'acc-personal', name: 'Drafts', kind: 'drafts', unreadCount: 0, totalCount: 1, icon: 'mdi-file-document-edit-outline' },
  { id: 'trash-personal', accountId: 'acc-personal', name: 'Trash', kind: 'trash', unreadCount: 0, totalCount: 0, icon: 'mdi-delete-outline' },
]

const threads: MailThread[] = [
  { id: 'thread-1', accountId: 'acc-work', subject: 'Milestone review for Linux MVP', messageIds: ['msg-1'], lastMessageAt: '2026-03-12T08:15:00.000Z', unreadCount: 1 },
  { id: 'thread-2', accountId: 'acc-work', subject: 'Material You palette options', messageIds: ['msg-2'], lastMessageAt: '2026-03-12T07:25:00.000Z', unreadCount: 0 },
  { id: 'thread-3', accountId: 'acc-work', subject: 'IMAP bridge spike notes', messageIds: ['msg-3'], lastMessageAt: '2026-03-11T18:42:00.000Z', unreadCount: 1 },
  { id: 'thread-4', accountId: 'acc-personal', subject: 'Dinner on Friday?', messageIds: ['msg-4'], lastMessageAt: '2026-03-12T06:10:00.000Z', unreadCount: 1 },
  { id: 'thread-5', accountId: 'acc-personal', subject: 'Flight receipt attached', messageIds: ['msg-5'], lastMessageAt: '2026-03-11T22:30:00.000Z', unreadCount: 0 },
]

let messages: MailMessage[] = [
  {
    id: 'msg-1',
    accountId: 'acc-work',
    folderId: 'inbox-work',
    threadId: 'thread-1',
    subject: 'Milestone review for Linux MVP',
    preview: 'The shell, routing, and Vuetify theme pass look ready for the first review.',
    body: '<p>Hi team,</p><p>The Linux-first desktop shell is ready for review. We now have the three-pane layout, mock multi-account flows, and a dynamic Material You inspired theme seeded from user color preferences.</p><p>For the next slice, I recommend wiring the inbox list through the desktop mail bridge so the repository boundary stays stable while we replace mocks later.</p><p>— Rowan</p>',
    from: 'Rowan Patel',
    fromEmail: 'rowan@mailstack.dev',
    to: ['hello@mailstack.dev'],
    cc: ['design@mailstack.dev'],
    sentAt: '2026-03-12T08:15:00.000Z',
    receivedAt: '2026-03-12T08:15:00.000Z',
    isRead: false,
    isStarred: true,
    hasAttachments: false,
    attachments: [],
    labels: ['MVP'],
  },
  {
    id: 'msg-2',
    accountId: 'acc-work',
    folderId: 'inbox-work',
    threadId: 'thread-2',
    subject: 'Material You palette options',
    preview: 'Sharing two seed colors that feel balanced on both light and dark surfaces.',
    body: '<p>I tested violet and teal as seed colors. Violet feels stronger for work mail, while teal keeps the reader calmer for longer sessions.</p><p>We can expose the seed as a user preference and persist only the seed plus density mode.</p>',
    from: 'Ava Kim',
    fromEmail: 'ava@mailstack.dev',
    to: ['hello@mailstack.dev'],
    cc: [],
    sentAt: '2026-03-12T07:25:00.000Z',
    receivedAt: '2026-03-12T07:25:00.000Z',
    isRead: true,
    isStarred: false,
    hasAttachments: false,
    attachments: [],
    labels: ['Theme'],
  },
  {
    id: 'msg-3',
    accountId: 'acc-work',
    folderId: 'inbox-work',
    threadId: 'thread-3',
    subject: 'IMAP bridge spike notes',
    preview: 'Attached are the protocol notes and rough command boundaries we discussed.',
    body: '<p>I mapped the protocol layer into provider::imap, provider::smtp, and sync orchestration. We can keep commands narrow and return serde-aligned domain structs back to the UI.</p>',
    from: 'Noah Silva',
    fromEmail: 'noah@mailstack.dev',
    to: ['hello@mailstack.dev'],
    cc: [],
    sentAt: '2026-03-11T18:42:00.000Z',
    receivedAt: '2026-03-11T18:42:00.000Z',
    isRead: false,
    isStarred: false,
    hasAttachments: true,
    attachments: [
      { id: 'att-1', fileName: 'imap-spike.pdf', mimeType: 'application/pdf', sizeBytes: 58231 },
    ],
    labels: ['Protocol'],
  },
  {
    id: 'msg-4',
    accountId: 'acc-personal',
    folderId: 'inbox-personal',
    threadId: 'thread-4',
    subject: 'Dinner on Friday?',
    preview: 'Still good for noodles this Friday night?',
    body: '<p>Hey,</p><p>Are you still free for dinner on Friday? I found a new noodle place near the station.</p><p>— Lin</p>',
    from: 'Lin',
    fromEmail: 'lin@example.com',
    to: ['iscccc@example.com'],
    cc: [],
    sentAt: '2026-03-12T06:10:00.000Z',
    receivedAt: '2026-03-12T06:10:00.000Z',
    isRead: false,
    isStarred: false,
    hasAttachments: false,
    attachments: [],
    labels: [],
  },
  {
    id: 'msg-5',
    accountId: 'acc-personal',
    folderId: 'sent-personal',
    threadId: 'thread-5',
    subject: 'Flight receipt attached',
    preview: 'Forwarding the receipt in case you need it for reimbursement.',
    body: '<p>Sending the flight receipt over now. Let me know if you also need the hotel invoice.</p>',
    from: 'iscccc',
    fromEmail: 'iscccc@example.com',
    to: ['travel@example.com'],
    cc: [],
    sentAt: '2026-03-11T22:30:00.000Z',
    receivedAt: '2026-03-11T22:30:00.000Z',
    isRead: true,
    isStarred: true,
    hasAttachments: true,
    attachments: [
      { id: 'att-2', fileName: 'receipt.pdf', mimeType: 'application/pdf', sizeBytes: 22104 },
    ],
    labels: ['Travel'],
  },
]

let drafts: DraftMessage[] = [
  {
    id: 'draft-1',
    accountId: 'acc-work',
    to: 'infra@mailstack.dev',
    cc: '',
    bcc: '',
    subject: 'Sync engine handoff notes',
    body: 'Starting a short handoff doc for the sync engine boundaries and retry model.',
  },
]

const syncStates = new Map<string, SyncStatus>([
  ['acc-work', { accountId: 'acc-work', state: 'syncing', message: 'Last sync completed 4 minutes ago', updatedAt: accounts[0].lastSyncedAt }],
  ['acc-personal', { accountId: 'acc-personal', state: 'idle', message: 'Mailbox is up to date', updatedAt: accounts[1].lastSyncedAt }],
])

const delay = (ms = 100) => new Promise((resolve) => setTimeout(resolve, ms))

const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T

const createDefaultFolders = (accountId: string): MailboxFolder[] => [
  { id: `inbox-${accountId}`, accountId, name: 'Inbox', kind: 'inbox', unreadCount: 0, totalCount: 0, icon: 'mdi-inbox-arrow-down' },
  { id: `starred-${accountId}`, accountId, name: 'Starred', kind: 'starred', unreadCount: 0, totalCount: 0, icon: 'mdi-star-outline' },
  { id: `sent-${accountId}`, accountId, name: 'Sent', kind: 'sent', unreadCount: 0, totalCount: 0, icon: 'mdi-send-outline' },
  {
    id: `drafts-${accountId}`,
    accountId,
    name: 'Drafts',
    kind: 'drafts',
    unreadCount: 0,
    totalCount: 0,
    icon: 'mdi-file-document-edit-outline',
  },
  { id: `trash-${accountId}`, accountId, name: 'Trash', kind: 'trash', unreadCount: 0, totalCount: 0, icon: 'mdi-delete-outline' },
]

const createInitialSyncStatus = (accountId: string, updatedAt: string): SyncStatus => ({
  accountId,
  state: 'idle',
  message: 'Mailbox is up to date',
  updatedAt,
})

const listMessagesForFolder = (accountId: string, folderId: string) => {
  const folder = folders.find((item) => item.id === folderId && item.accountId === accountId)

  if (!folder) {
    return []
  }

  if (folder.kind === 'starred') {
    return messages.filter((message) => message.accountId === accountId && message.isStarred)
  }

  return messages.filter((message) => message.accountId === accountId && message.folderId === folderId)
}

export const mockMailRepository: MailRepository = {
  async listAccounts(): Promise<MailAccount[]> {
    await delay()
    return clone(accounts)
  },

  async createAccount(draft: AccountSetupDraft): Promise<MailAccount> {
    await delay()
    const lastSyncedAt = new Date().toISOString()
    const nextAccount: MailAccount = {
      id: `acc-${crypto.randomUUID()}`,
      name: draft.displayName.trim() || draft.email,
      email: draft.email,
      provider: draft.provider,
      color: '#5B8DEF',
      initials: (draft.displayName || draft.email)
        .split(/\s+/)
        .filter(Boolean)
        .slice(0, 2)
        .map((part) => part[0]?.toUpperCase() ?? '')
        .join('') || 'NA',
      unreadCount: 0,
      status: 'connected',
      lastSyncedAt,
    }

    accounts.unshift(nextAccount)
    folders.unshift(...createDefaultFolders(nextAccount.id))
    syncStates.set(nextAccount.id, createInitialSyncStatus(nextAccount.id, lastSyncedAt))
    return clone(nextAccount)
  },

  async listFolders(accountId: string): Promise<MailboxFolder[]> {
    await delay()
    return clone(folders.filter((folder) => folder.accountId === accountId))
  },

  async listMessages(accountId: string, folderId: string): Promise<MailMessage[]> {
    await delay()
    return clone(listMessagesForFolder(accountId, folderId).sort((a, b) => b.receivedAt.localeCompare(a.receivedAt)))
  },

  async getMessage(accountId: string, messageId: string): Promise<MailMessage | null> {
    await delay()
    const message = messages.find((item) => item.accountId === accountId && item.id === messageId)
    return message ? clone(message) : null
  },

  async saveDraft(draft: DraftMessage): Promise<DraftMessage> {
    await delay()
    const existing = drafts.find((item) => item.id === draft.id)

    if (existing) {
      Object.assign(existing, draft)
      return clone(existing)
    }

    const nextDraft = { ...draft, id: draft.id || `draft-${drafts.length + 1}` }
    drafts = [nextDraft, ...drafts]
    return clone(nextDraft)
  },

  async sendMessage(draft: DraftMessage): Promise<{ ok: true; queuedAt: string }> {
    await delay()
    drafts = drafts.filter((item) => item.id !== draft.id)
    return { ok: true as const, queuedAt: new Date().toISOString() }
  },

  async toggleStar(accountId: string, messageId: string): Promise<MailMessage | null> {
    await delay()
    const message = messages.find((item) => item.accountId === accountId && item.id === messageId)

    if (!message) {
      return null
    }

    message.isStarred = !message.isStarred
    return clone(message)
  },

  async toggleRead(accountId: string, messageId: string): Promise<MailMessage | null> {
    await delay()
    const message = messages.find((item) => item.accountId === accountId && item.id === messageId)

    if (!message) {
      return null
    }

    message.isRead = !message.isRead
    return clone(message)
  },

  async deleteMessage(accountId: string, messageId: string): Promise<void> {
    await delay()
    const message = messages.find((item) => item.accountId === accountId && item.id === messageId)

    if (!message) {
      return
    }

    const trashFolder = folders.find((folder) => folder.accountId === accountId && folder.kind === 'trash')

    if (trashFolder) {
      message.folderId = trashFolder.id
      message.isRead = true
    }
  },

  async syncAccount(accountId: string): Promise<SyncStatus> {
    await delay(200)
    const status: SyncStatus = {
      accountId,
      state: 'idle',
      message: 'Sync completed successfully',
      updatedAt: new Date().toISOString(),
    }
    syncStates.set(accountId, status)
    return clone(status)
  },

  async getMailboxBundle(accountId: string): Promise<MailboxBundle> {
    await delay()
    return clone({
      accountId,
      folders: folders.filter((folder) => folder.accountId === accountId),
      messages: messages.filter((message) => message.accountId === accountId),
      threads: threads.filter((thread) => thread.accountId === accountId),
      syncStatus: syncStates.get(accountId) ?? createInitialSyncStatus(accountId, new Date().toISOString()),
    } satisfies MailboxBundle)
  },
}
