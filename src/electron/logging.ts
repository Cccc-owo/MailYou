import { appendFileSync, existsSync, mkdirSync, renameSync, statSync, unlinkSync } from 'node:fs'
import { basename, dirname, extname, join } from 'node:path'
import { formatWithOptions } from 'node:util'

type ConsoleMethod = 'debug' | 'info' | 'log' | 'warn' | 'error'
type LogLevel = 'debug' | 'info' | 'warn' | 'error'

const APP_DIR_NAME = 'MailYou'
const LOG_DIR_NAME = 'logs'
const LOG_MAX_BYTES = 2 * 1024 * 1024
const LOG_MAX_ROTATED_FILES = 5

let initialized = false
let logFilePath: string | null = null

const LOG_LEVEL_ORDER: Record<LogLevel, number> = {
  debug: 10,
  info: 20,
  warn: 30,
  error: 40,
}

const originalConsole = {
  debug: console.debug.bind(console),
  info: console.info.bind(console),
  log: console.log.bind(console),
  warn: console.warn.bind(console),
  error: console.error.bind(console),
}

const timestamp = () => new Date().toISOString()

const currentLogFileName = () => `app-${timestamp().slice(0, 10)}.log`

const REDACTED = '[REDACTED]'

const normalizeLevel = (method: ConsoleMethod): LogLevel => (method === 'log' ? 'info' : method)

const resolveMinimumLogLevel = (): LogLevel => {
  const raw = process.env.MAILYOU_LOG_LEVEL?.trim().toLowerCase()
  if (raw === 'debug' || raw === 'info' || raw === 'warn' || raw === 'error') {
    return raw
  }
  return 'info'
}

const shouldWriteLevel = (method: ConsoleMethod) =>
  LOG_LEVEL_ORDER[normalizeLevel(method)] >= LOG_LEVEL_ORDER[resolveMinimumLogLevel()]

const resolveDataRoot = () => {
  const custom = process.env.MAILYOU_DATA_DIR?.trim()
  if (custom) {
    return custom
  }

  if (process.platform === 'win32') {
    const appData = process.env.APPDATA?.trim()
    if (!appData) {
      throw new Error('APPDATA environment variable not set')
    }
    return appData
  }

  const xdgDataHome = process.env.XDG_DATA_HOME?.trim()
  if (xdgDataHome) {
    return xdgDataHome
  }

  const home = process.env.HOME?.trim()
  if (!home) {
    throw new Error('HOME environment variable not set')
  }

  return join(home, '.local', 'share')
}

const ensureLogFilePath = () => {
  if (logFilePath) {
    return logFilePath
  }

  const logDir = join(resolveDataRoot(), APP_DIR_NAME, LOG_DIR_NAME)
  mkdirSync(logDir, { recursive: true })
  logFilePath = join(logDir, currentLogFileName())
  return logFilePath
}

const getRotatedLogPath = (basePath: string, index: number) => {
  const extension = extname(basePath)
  const name = basename(basePath, extension)
  return join(dirname(basePath), `${name}.${index}${extension}`)
}

const rotateLogFileIfNeeded = (filePath: string, nextEntrySize: number) => {
  try {
    const size = existsSync(filePath) ? statSync(filePath).size : 0
    if (size + nextEntrySize <= LOG_MAX_BYTES) {
      return
    }

    const oldestPath = getRotatedLogPath(filePath, LOG_MAX_ROTATED_FILES)
    if (existsSync(oldestPath)) {
      unlinkSync(oldestPath)
    }

    for (let index = LOG_MAX_ROTATED_FILES - 1; index >= 1; index -= 1) {
      const sourcePath = getRotatedLogPath(filePath, index)
      if (!existsSync(sourcePath)) {
        continue
      }

      renameSync(sourcePath, getRotatedLogPath(filePath, index + 1))
    }

    if (existsSync(filePath)) {
      renameSync(filePath, getRotatedLogPath(filePath, 1))
    }
  } catch (error) {
    originalConsole.error(`failed to rotate log file: ${formatArgs([error])}`)
  }
}

const formatArgs = (args: unknown[]) =>
  formatWithOptions({ colors: false, depth: 6, breakLength: 120 }, ...args)

const redactUrlSecrets = (value: string) => {
  try {
    const url = new URL(value)
    let changed = false
    for (const key of ['code', 'state', 'access_token', 'refresh_token', 'id_token', 'token']) {
      if (url.searchParams.has(key)) {
        url.searchParams.set(key, REDACTED)
        changed = true
      }
    }

    return changed ? url.toString() : value
  } catch {
    return value
  }
}

const redactSensitiveText = (value: string) =>
  redactUrlSecrets(value)
    .replace(
      /\b(authorization|proxy-authorization)\s*:\s*(bearer|basic)\s+([^\s,;]+)/gi,
      (_match, key: string, scheme: string) => `${key}: ${scheme} ${REDACTED}`,
    )
    .replace(
      /(["']?)(password|currentPassword|newPassword|passphrase|secret|accessToken|refreshToken|idToken|token|code|authorization)\1\s*[:=]\s*(['"]?)([^'",\s}]+)\3/gi,
      (_match, quote: string, key: string) => `${quote}${key}${quote}: ${REDACTED}`,
    )
    .replace(/\b(code|state|access_token|refresh_token|id_token|token)=([^&\s]+)/gi, `$1=${REDACTED}`)
    .replace(/\bBearer\s+([A-Za-z0-9\-._~+/]+=*)/gi, `Bearer ${REDACTED}`)
    .replace(
      /\b([A-Z0-9._%+-]{1,64})@([A-Z0-9.-]+\.[A-Z]{2,})\b/gi,
      (_match, local: string, domain: string) => {
        const prefix = local.slice(0, Math.min(2, local.length))
        return `${prefix}${local.length > prefix.length ? '***' : ''}@${domain}`
      },
    )
    .replace(/\bacc-[a-z0-9-]{6,}\b/gi, (match) => `${match.slice(0, 8)}***`)
    .replace(/\b(account|accountId)\s*[:=]\s*([^\s,;]+)/gi, (_match, key: string) => `${key}=${REDACTED}`)
    .replace(/\b(user|username|email)\s*[:=]\s*([^\s,;]+)/gi, (_match, key: string) => `${key}=${REDACTED}`)

const sanitizeLogMessage = (message: string) => redactSensitiveText(message)

const formatLevel = (method: ConsoleMethod) => normalizeLevel(method).toUpperCase().padEnd(5, ' ')

const writeLog = (method: ConsoleMethod, args: unknown[]) => {
  if (!shouldWriteLevel(method)) {
    return
  }

  const message = sanitizeLogMessage(formatArgs(args))
  const lines = message.split(/\r?\n/)
  const prefix = `[${timestamp()}] [${formatLevel(method)}]`
  const output = lines.map((line) => `${prefix} ${line}`).join('\n')
  const serialized = `${output}\n`

  try {
    const filePath = ensureLogFilePath()
    rotateLogFileIfNeeded(filePath, Buffer.byteLength(serialized, 'utf8'))
    appendFileSync(filePath, serialized, 'utf8')
  } catch (error) {
    originalConsole.error(`failed to write log file: ${formatArgs([error])}`)
  }

  originalConsole[method](output)
}

export const initializeMainProcessLogging = () => {
  if (initialized) {
    return
  }

  initialized = true

  console.debug = (...args: unknown[]) => writeLog('debug', args)
  console.info = (...args: unknown[]) => writeLog('info', args)
  console.log = (...args: unknown[]) => writeLog('log', args)
  console.warn = (...args: unknown[]) => writeLog('warn', args)
  console.error = (...args: unknown[]) => writeLog('error', args)

  process.on('uncaughtException', (error) => {
    console.error('[process] uncaughtException', error)
  })
  process.on('unhandledRejection', (reason) => {
    console.error('[process] unhandledRejection', reason)
  })

  console.info(`[logger] writing main-process logs to ${ensureLogFilePath()} (level=${resolveMinimumLogLevel()})`)
}

export const getMainProcessLogFilePath = () => ensureLogFilePath()
