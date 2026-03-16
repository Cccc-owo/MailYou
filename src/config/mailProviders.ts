import type { MailProviderPreset, OAuthProviderAvailability } from '@/types/account'

export const MAIL_PROVIDER_PRESETS: Record<string, MailProviderPreset> = {
  gmail: { provider: 'Gmail', authMode: 'oauth', oauthProvider: 'gmail', incomingHost: 'imap.gmail.com', incomingPort: 993, outgoingHost: 'smtp.gmail.com', outgoingPort: 465, useTls: true },
  'gmail.com': { provider: 'Gmail', authMode: 'oauth', oauthProvider: 'gmail', incomingHost: 'imap.gmail.com', incomingPort: 993, outgoingHost: 'smtp.gmail.com', outgoingPort: 465, useTls: true },
  'googlemail.com': { provider: 'Gmail', authMode: 'oauth', oauthProvider: 'gmail', incomingHost: 'imap.gmail.com', incomingPort: 993, outgoingHost: 'smtp.gmail.com', outgoingPort: 465, useTls: true },
  outlook: { provider: 'Outlook', authMode: 'oauth', oauthProvider: 'outlook', incomingHost: 'outlook.office365.com', incomingPort: 993, outgoingHost: 'smtp-mail.outlook.com', outgoingPort: 587, useTls: true },
  'outlook.com': { provider: 'Outlook', authMode: 'oauth', oauthProvider: 'outlook', incomingHost: 'outlook.office365.com', incomingPort: 993, outgoingHost: 'smtp-mail.outlook.com', outgoingPort: 587, useTls: true },
  'hotmail.com': { provider: 'Outlook', authMode: 'oauth', oauthProvider: 'outlook', incomingHost: 'outlook.office365.com', incomingPort: 993, outgoingHost: 'smtp-mail.outlook.com', outgoingPort: 587, useTls: true },
  'live.com': { provider: 'Outlook', authMode: 'oauth', oauthProvider: 'outlook', incomingHost: 'outlook.office365.com', incomingPort: 993, outgoingHost: 'smtp-mail.outlook.com', outgoingPort: 587, useTls: true },
  icloud: { provider: 'iCloud Mail', authMode: 'oauth', oauthProvider: 'icloud', incomingHost: 'imap.mail.me.com', incomingPort: 993, outgoingHost: 'smtp.mail.me.com', outgoingPort: 587, useTls: true },
  'icloud.com': { provider: 'iCloud Mail', authMode: 'oauth', oauthProvider: 'icloud', incomingHost: 'imap.mail.me.com', incomingPort: 993, outgoingHost: 'smtp.mail.me.com', outgoingPort: 587, useTls: true },
  'me.com': { provider: 'iCloud Mail', authMode: 'oauth', oauthProvider: 'icloud', incomingHost: 'imap.mail.me.com', incomingPort: 993, outgoingHost: 'smtp.mail.me.com', outgoingPort: 587, useTls: true },
  'mac.com': { provider: 'iCloud Mail', authMode: 'oauth', oauthProvider: 'icloud', incomingHost: 'imap.mail.me.com', incomingPort: 993, outgoingHost: 'smtp.mail.me.com', outgoingPort: 587, useTls: true },
  'qq.com': { provider: 'QQ', authMode: 'password', incomingHost: 'imap.qq.com', incomingPort: 993, outgoingHost: 'smtp.qq.com', outgoingPort: 465, useTls: true },
  'foxmail.com': { provider: 'QQ', authMode: 'password', incomingHost: 'imap.qq.com', incomingPort: 993, outgoingHost: 'smtp.qq.com', outgoingPort: 465, useTls: true },
  '163.com': { provider: 'NetEase 163', authMode: 'password', incomingHost: 'imap.163.com', incomingPort: 993, outgoingHost: 'smtp.163.com', outgoingPort: 465, useTls: true },
  '126.com': { provider: 'NetEase 126', authMode: 'password', incomingHost: 'imap.126.com', incomingPort: 993, outgoingHost: 'smtp.126.com', outgoingPort: 465, useTls: true },
  'yeah.net': { provider: 'NetEase Yeah', authMode: 'password', incomingHost: 'imap.yeah.net', incomingPort: 993, outgoingHost: 'smtp.yeah.net', outgoingPort: 465, useTls: true },
}

export const FALLBACK_OAUTH_PROVIDERS: OAuthProviderAvailability[] = [
  { id: 'gmail', label: 'Gmail', supportsDirect: false, supportsProxy: true },
  { id: 'outlook', label: 'Outlook', supportsDirect: false, supportsProxy: true },
  { id: 'icloud', label: 'iCloud Mail', supportsDirect: false, supportsProxy: true },
]

