export const MAILYOU_DEV_SERVER_URL_ENV = 'VITE_DEV_SERVER_URL'
export const MAILYOU_OZONE_PLATFORM_HINT_ENV = 'MAILYOU_OZONE_PLATFORM_HINT'
export const MAILYOU_ENABLE_DEV_PROTOCOL_CLIENT_ENV = 'MAILYOU_ENABLE_DEV_PROTOCOL_CLIENT'

export const MAILYOU_DEFAULT_OZONE_PLATFORM_HINT = 'auto'

export const isMailYouDevServerEnabled = (env = process.env) =>
  Boolean(env[MAILYOU_DEV_SERVER_URL_ENV])

export const getMailYouDevServerUrl = (env = process.env) => env[MAILYOU_DEV_SERVER_URL_ENV]

export const getMailYouOzonePlatformHint = (env = process.env) =>
  env[MAILYOU_OZONE_PLATFORM_HINT_ENV]?.trim() || MAILYOU_DEFAULT_OZONE_PLATFORM_HINT

export const isMailYouDevProtocolClientEnabled = (env = process.env) =>
  env[MAILYOU_ENABLE_DEV_PROTOCOL_CLIENT_ENV] === 'true'
