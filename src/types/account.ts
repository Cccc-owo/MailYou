export type AccountStatus = 'connected' | 'syncing' | 'attention'

export interface MailAccount {
  id: string
  name: string
  email: string
  provider: string
  incomingProtocol: string
  color: string
  initials: string
  unreadCount: number
  status: AccountStatus
  lastSyncedAt: string
}

export interface AccountSetupDraft {
  displayName: string
  email: string
  provider: string
  incomingProtocol: string
  incomingHost: string
  incomingPort: number
  outgoingHost: string
  outgoingPort: number
  username: string
  password: string
  useTls: boolean
}
