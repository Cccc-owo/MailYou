import { shell } from 'electron'
import { createHash, randomBytes } from 'node:crypto'
import {
  getMailYouOAuthProxyUrl,
  getMailYouOAuthProxyToken,
  MAILYOU_OAUTH_CALLBACK_PROTOCOL,
  OAUTH_PROVIDER_CONFIGS,
} from '@/config/oauth'
import type { OAuthAuthorizationRequest, OAuthAuthorizationResult, OAuthProviderId } from '@/types/account'

interface ProxyAuthUrlResponse {
  authUrl: string
  state: string
}

interface CallbackPayload {
  code?: string
  state?: string
  error?: string
  errorDescription?: string
}

interface DirectTokenResponse {
  access_token: string
  refresh_token?: string
  expires_in: number
}

const CALLBACK_TIMEOUT_MS = 180_000

let pendingAuthorization:
  | {
      expectedState: string
      resolve: (payload: CallbackPayload) => void
      reject: (error: Error) => void
      timer: NodeJS.Timeout
    }
  | null = null

export const authorizeOAuth = async (
  request: OAuthAuthorizationRequest,
): Promise<OAuthAuthorizationResult> => {
  if (pendingAuthorization) {
    throw new Error('Another OAuth sign-in is already in progress')
  }

  const provider = OAUTH_PROVIDER_CONFIGS[request.provider]
  if (!provider) {
    throw new Error(`Unsupported OAuth provider: ${request.provider}`)
  }

  const codeVerifier = createCodeVerifier()
  const codeChallenge = createCodeChallenge(codeVerifier)
  const clientState = randomBytes(16).toString('hex')

  let authUrl: string
  let expectedState: string

  if (request.source === 'proxy') {
    const proxyBaseUrl = getMailYouOAuthProxyUrl()
    const response = await fetchJson<ProxyAuthUrlResponse>(
      `${proxyBaseUrl}/api/auth-url?${new URLSearchParams({
        provider: request.provider,
        state: clientState,
        codeChallenge,
        codeChallengeMethod: 'S256',
      }).toString()}`,
    )
    authUrl = response.authUrl
    expectedState = response.state
  } else {
    if (request.provider === 'icloud') {
      throw new Error('iCloud direct authorization is not supported in the desktop app')
    }

    const redirectUri = requireEnv(provider.redirectUriEnv!, `${provider.label} redirect URI`)
    const clientId = requireEnv(provider.clientIdEnv!, `${provider.label} client ID`)
    const url = new URL(provider.authUrl)
    url.searchParams.set('client_id', clientId)
    url.searchParams.set('redirect_uri', redirectUri)
    url.searchParams.set('response_type', 'code')
    url.searchParams.set('scope', provider.scopes.join(' '))
    url.searchParams.set('state', clientState)
    url.searchParams.set('code_challenge', codeChallenge)
    url.searchParams.set('code_challenge_method', 'S256')
    url.searchParams.set('access_type', 'offline')
    url.searchParams.set('prompt', 'consent')
    for (const [key, value] of Object.entries(provider.authParams ?? {})) {
      url.searchParams.set(key, value)
    }
    authUrl = url.toString()
    expectedState = clientState
  }

  const callback = await waitForOAuthCallback(expectedState, async () => {
    await shell.openExternal(authUrl)
  })

  if (callback.error) {
    throw new Error(callback.errorDescription || callback.error)
  }
  if (!callback.code || !callback.state) {
    throw new Error('OAuth provider did not return an authorization code')
  }

  if (request.source === 'proxy') {
    const proxyBaseUrl = getMailYouOAuthProxyUrl()
    return await postJson<OAuthAuthorizationResult>(`${proxyBaseUrl}/api/token`, {
      provider: request.provider,
      code: callback.code,
      state: callback.state,
      codeVerifier,
    })
  }

  const redirectUri = requireEnv(provider.redirectUriEnv!, `${provider.label} redirect URI`)
  const clientId = requireEnv(provider.clientIdEnv!, `${provider.label} client ID`)
  const clientSecret = requireEnv(provider.clientSecretEnv!, `${provider.label} client secret`)
  const tokens = await postForm<DirectTokenResponse>(provider.tokenUrl, {
    client_id: clientId,
    client_secret: clientSecret,
    code: callback.code,
    grant_type: 'authorization_code',
    redirect_uri: redirectUri,
    code_verifier: codeVerifier,
  })

  if (typeof tokens.access_token !== 'string' || !Number.isFinite(tokens.expires_in)) {
    throw new Error('OAuth provider returned an invalid token response')
  }

  return {
    accessToken: tokens.access_token,
    refreshToken: tokens.refresh_token ?? '',
    expiresAt: new Date(Date.now() + tokens.expires_in * 1000).toISOString(),
  }
}

export const handleOAuthCallbackUrl = (rawUrl: string) => {
  const payload = extractCallbackPayload(rawUrl)
  if (!pendingAuthorization || payload.state !== pendingAuthorization.expectedState) {
    return false
  }

  clearTimeout(pendingAuthorization.timer)
  const { resolve } = pendingAuthorization
  pendingAuthorization = null
  resolve(payload)
  return true
}

const waitForOAuthCallback = async (
  expectedState: string,
  launchBrowser: () => Promise<void>,
): Promise<CallbackPayload> => {
  const callbackPromise = new Promise<CallbackPayload>((resolve, reject) => {
    const timer = setTimeout(() => {
      pendingAuthorization = null
      reject(new Error('OAuth sign-in timed out'))
    }, CALLBACK_TIMEOUT_MS)

    pendingAuthorization = { expectedState, resolve, reject, timer }
  })

  try {
    await launchBrowser()
    return await callbackPromise
  } catch (error) {
    if (pendingAuthorization?.expectedState === expectedState) {
      clearTimeout(pendingAuthorization.timer)
      pendingAuthorization = null
    }
    throw error
  }
}

const extractCallbackPayload = (rawUrl: string): CallbackPayload => {
  const url = new URL(rawUrl)
  const query = url.searchParams
  return {
    code: query.get('code') ?? undefined,
    state: query.get('state') ?? undefined,
    error: query.get('error') ?? undefined,
    errorDescription: query.get('error_description') ?? undefined,
  }
}

const requireEnv = (name: string, label: string) => {
  const value = process.env[name]?.trim()
  if (!value) {
    throw new Error(`${label} is not configured`)
  }
  return value
}

const createCodeVerifier = () => randomBytes(32).toString('base64url')

const createCodeChallenge = (codeVerifier: string) =>
  createHash('sha256').update(codeVerifier).digest('base64url')

const fetchJson = async <T>(url: string): Promise<T> => {
  const response = await fetch(url)
  if (!response.ok) {
    throw new Error(await readErrorResponse(response))
  }
  return (await response.json()) as T
}

const postJson = async <T>(url: string, body: Record<string, string>): Promise<T> => {
  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${getMailYouOAuthProxyToken()}`,
    },
    body: JSON.stringify(body),
  })
  if (!response.ok) {
    throw new Error(await readErrorResponse(response))
  }
  return (await response.json()) as T
}

const postForm = async <T>(url: string, body: Record<string, string>): Promise<T> => {
  const response = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams(body),
  })
  if (!response.ok) {
    throw new Error(await readErrorResponse(response))
  }
  return (await response.json()) as T
}

const readErrorResponse = async (response: Response) => {
  try {
    const payload = (await response.json()) as {
      error?: { message?: string } | string
      message?: string
    }
    if (typeof payload.error === 'string' && payload.error.trim()) {
      return payload.error
    }
    if (payload.error && typeof payload.error === 'object' && typeof payload.error.message === 'string') {
      return payload.error.message
    }
    return payload.message || `Request failed with status ${response.status}`
  } catch {
    return `Request failed with status ${response.status}`
  }
}

export const getExpectedOAuthRedirectProtocol = () => MAILYOU_OAUTH_CALLBACK_PROTOCOL
