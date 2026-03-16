import type { OAuthProviderId } from '@/types/account'

export interface OAuthProviderConfig {
  label: string
  authUrl: string
  tokenUrl: string
  scopes: string[]
  clientIdEnv?: string
  clientSecretEnv?: string
  redirectUriEnv?: string
  authParams?: Record<string, string>
}

export const MAILYOU_OAUTH_CALLBACK_PROTOCOL = 'mailyou://oauth/callback'
export const MAILYOU_OAUTH_PROXY_TOKEN_ENV = 'MAILYOU_OAUTH_PROXY_TOKEN'
export const MAILYOU_OAUTH_PROXY_AUTH_TOKEN = 'Lu6WVgtL31TkaXWVeVBIaB8T8CsU3jMfXoxbpomAuas5hF5wpOx5IWfdUiokkc5G'
export const MAILYOU_OAUTH_PROXY_BASE_URL = 'https://oauth2-proxy.iscccc.cc'
export const MAILYOU_OAUTH_PROXY_URL_ENV = 'MAILYOU_OAUTH_PROXY_URL'

export const OAUTH_PROVIDER_CONFIGS: Record<OAuthProviderId, OAuthProviderConfig> = {
  gmail: {
    label: 'Gmail',
    authUrl: 'https://accounts.google.com/o/oauth2/v2/auth',
    tokenUrl: 'https://oauth2.googleapis.com/token',
    scopes: ['https://mail.google.com/'],
    clientIdEnv: 'GMAIL_CLIENT_ID',
    clientSecretEnv: 'GMAIL_CLIENT_SECRET',
    redirectUriEnv: 'GMAIL_REDIRECT_URI',
  },
  outlook: {
    label: 'Outlook',
    authUrl: 'https://login.microsoftonline.com/common/oauth2/v2.0/authorize',
    tokenUrl: 'https://login.microsoftonline.com/common/oauth2/v2.0/token',
    scopes: [
      'https://outlook.office.com/IMAP.AccessAsUser.All',
      'https://outlook.office.com/SMTP.Send',
      'offline_access',
    ],
    clientIdEnv: 'OUTLOOK_CLIENT_ID',
    clientSecretEnv: 'OUTLOOK_CLIENT_SECRET',
    redirectUriEnv: 'OUTLOOK_REDIRECT_URI',
  },
  icloud: {
    label: 'iCloud Mail',
    authUrl: 'https://appleid.apple.com/auth/authorize',
    tokenUrl: 'https://appleid.apple.com/auth/token',
    scopes: ['email'],
  },
}

export const getMailYouOAuthProxyUrl = (env = process.env) =>
  (env[MAILYOU_OAUTH_PROXY_URL_ENV]?.trim() || MAILYOU_OAUTH_PROXY_BASE_URL).replace(/\/+$/, '')

export const getMailYouOAuthProxyToken = (env = process.env) =>
  env[MAILYOU_OAUTH_PROXY_TOKEN_ENV]?.trim() || MAILYOU_OAUTH_PROXY_AUTH_TOKEN
