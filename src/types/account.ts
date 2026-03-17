export type AccountStatus = 'connected' | 'syncing' | 'attention'
export type AccountAuthMode = 'password' | 'oauth'
export type OAuthProviderId = 'gmail' | 'outlook' | 'icloud'
export type OAuthSource = 'direct' | 'proxy'

export interface MailIdentity {
  id: string
  name: string
  email: string
  replyTo?: string | null
  signature?: string | null
  isDefault: boolean
}

export interface OAuthProviderAvailability {
  id: OAuthProviderId
  label: string
  supportsDirect: boolean
  supportsProxy: boolean
}

export interface MailProviderPreset {
  provider: string
  authMode: AccountAuthMode
  oauthProvider?: OAuthProviderId
  incomingHost: string
  incomingPort: number
  outgoingHost: string
  outgoingPort: number
  useTls: boolean
}

export interface OAuthAuthorizationRequest {
  provider: OAuthProviderId
  source: OAuthSource
}

export interface OAuthAuthorizationResult {
  accessToken: string
  refreshToken: string
  expiresAt: string
}

export interface MailAccount {
  id: string
  name: string
  email: string
  provider: string
  incomingProtocol: string
  authMode: AccountAuthMode
  oauthProvider?: OAuthProviderId | null
  oauthSource?: OAuthSource | null
  color: string
  initials: string
  unreadCount: number
  status: AccountStatus
  lastSyncedAt: string
  identities: MailIdentity[]
}

export interface AccountSetupDraft {
  displayName: string
  email: string
  provider: string
  authMode: AccountAuthMode
  incomingProtocol: string
  incomingHost: string
  incomingPort: number
  outgoingHost: string
  outgoingPort: number
  username: string
  password: string
  useTls: boolean
  oauthProvider: OAuthProviderId | null
  oauthSource: OAuthSource | null
  accessToken: string
  refreshToken: string
  tokenExpiresAt: string
  identities: MailIdentity[]
}
