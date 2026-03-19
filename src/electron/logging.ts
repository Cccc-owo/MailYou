import { appendFileSync, existsSync, mkdirSync, renameSync, statSync, unlinkSync } from 'node:fs'
import { basename, dirname, extname, join } from 'node:path'
import { formatWithOptions } from 'node:util'

type ConsoleMethod = 'debug' | 'info' | 'log' | 'warn' | 'error'
type LogLevel = 'debug' | 'info' | 'warn' | 'error'
type LogFieldValue =
  | string
  | number
  | boolean
  | null
  | LogFieldValue[]
  | { [key: string]: LogFieldValue }

export type LogFields = Record<string, LogFieldValue>

type StructuredLogEntry = {
  ts: string
  level: LogLevel
  event: string
  msg?: string
  fields?: LogFields
}

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
    originalConsole.error(
      formatConsoleEntry({
        ts: timestamp(),
        level: 'error',
        event: 'logger.rotate_failed',
        fields: {
          error: sanitizeLogValue(error),
        },
      }),
    )
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

const sanitizeLogValue = (value: unknown): LogFieldValue => {
  if (value == null) {
    return null
  }

  if (typeof value === 'string') {
    return sanitizeLogMessage(value)
  }

  if (typeof value === 'number' || typeof value === 'boolean') {
    return value
  }

  if (value instanceof Error) {
    return {
      name: sanitizeLogMessage(value.name),
      message: sanitizeLogMessage(value.message),
    }
  }

  if (Array.isArray(value)) {
    return value.map((item) => sanitizeLogValue(item))
  }

  if (typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(([key, item]) => [key, sanitizeLogValue(item)]),
    )
  }

  return sanitizeLogMessage(String(value))
}

const formatLevel = (method: ConsoleMethod) => normalizeLevel(method).toUpperCase()

const formatFieldValue = (value: LogFieldValue) => {
  if (Array.isArray(value) || (value && typeof value === 'object')) {
    return JSON.stringify(value)
  }

  return String(value)
}

const formatConsoleEntry = (entry: StructuredLogEntry) => {
  const prefix = `[${entry.ts}] [${entry.level.toUpperCase()}] ${entry.event}`
  const messagePart = entry.msg ? ` ${entry.msg}` : ''
  const fieldPart = entry.fields
    ? Object.entries(entry.fields)
        .map(([key, value]) => `${key}=${formatFieldValue(value)}`)
        .join(' ')
    : ''

  return `${prefix}${messagePart}${fieldPart ? ` ${fieldPart}` : ''}`
}

const serializeEntry = (entry: StructuredLogEntry) => `${formatConsoleEntry(entry)}\n`

const asNumber = (value: LogFieldValue | undefined) =>
  typeof value === 'number' && Number.isFinite(value) ? value : null

const asString = (value: LogFieldValue | undefined) => (typeof value === 'string' ? value : null)

const shouldKeepRustStderrInfo = (entry: StructuredLogEntry) => {
  const component = asString(entry.fields?.component) ?? ''
  const message = asString(entry.fields?.message) ?? asString(entry.fields?.line) ?? entry.msg ?? ''
  if (!message) {
    return false
  }

  if (component === 'imap') {
    return (
      /^syncing account /i.test(message) ||
      /^fetched \d+ folders, \d+ messages in /i.test(message)
    )
  }

  return false
}

const shouldDemoteRustStderrInfo = (entry: StructuredLogEntry) => {
  const component = asString(entry.fields?.component) ?? ''
  const message = asString(entry.fields?.message) ?? asString(entry.fields?.line) ?? entry.msg ?? ''
  if (!message) {
    return false
  }

  if (component === 'store') {
    return true
  }

  if (component === 'backend' && /^req #\d+ .+ → ok \([^)]+\)$/i.test(message)) {
    return true
  }

  if (component === 'backend' && /^req #\d+ .+$/i.test(message)) {
    return true
  }

  return (
    /^starting\.\.\.$/i.test(message) ||
    /^initialized provider /i.test(message) ||
    /^ready, reading stdin$/i.test(message) ||
    /^loading initial state/i.test(message) ||
    /^loaded \d+ accounts from disk$/i.test(message) ||
    /^loaded \d+ contacts, \d+ groups from disk$/i.test(message) ||
    /^persisted \(/i.test(message) ||
    /^req #\d+ .+$/i.test(message)
  )
}

const resolveEffectiveLevel = (entry: StructuredLogEntry): LogLevel => {
  if (entry.level !== 'info') {
    return entry.level
  }

  if (
    entry.event === 'logger.initialized' ||
    entry.event === 'rust.process_starting' ||
    entry.event === 'rust.process_started'
  ) {
    return 'debug'
  }

  if (entry.event === 'rust.process_exit' && entry.fields?.expected === true) {
    return 'debug'
  }

  if (entry.event === 'ipc.response' && entry.fields?.status === 'ok') {
    return 'debug'
  }

  if (entry.event === 'rpc.response' && entry.fields?.status === 'ok') {
    const elapsedMs = asNumber(entry.fields?.elapsed_ms)
    if (elapsedMs === null || elapsedMs < 500) {
      return 'debug'
    }
  }

  if (entry.event.startsWith('rust.') && entry.event.endsWith('.stderr')) {
    if (shouldKeepRustStderrInfo(entry)) {
      return 'info'
    }

    if (shouldDemoteRustStderrInfo(entry)) {
      return 'debug'
    }

    return 'debug'
  }

  return entry.level
}

const writeEntry = (entry: StructuredLogEntry) => {
  const effectiveLevel = resolveEffectiveLevel(entry)
  if (LOG_LEVEL_ORDER[effectiveLevel] < LOG_LEVEL_ORDER[resolveMinimumLogLevel()]) {
    return
  }

  const sanitizedEntry: StructuredLogEntry = {
    ts: entry.ts,
    level: effectiveLevel,
    event: sanitizeLogMessage(entry.event),
    msg: entry.msg ? sanitizeLogMessage(entry.msg) : undefined,
    fields: entry.fields ? (sanitizeLogValue(entry.fields) as LogFields) : undefined,
  }
  const serialized = serializeEntry(sanitizedEntry)
  const output = formatConsoleEntry(sanitizedEntry)

  try {
    const filePath = ensureLogFilePath()
    rotateLogFileIfNeeded(filePath, Buffer.byteLength(serialized, 'utf8'))
    appendFileSync(filePath, serialized, 'utf8')
  } catch (error) {
    originalConsole.error(
      formatConsoleEntry({
        ts: timestamp(),
        level: 'error',
        event: 'logger.write_failed',
        fields: {
          error: sanitizeLogValue(error),
        },
      }),
    )
  }

  originalConsole[sanitizedEntry.level](output)
}

const writeLog = (method: ConsoleMethod, args: unknown[]) => {
  writeEntry({
    ts: timestamp(),
    level: normalizeLevel(method),
    event: 'console',
    msg: formatArgs(args),
  })
}

export const logEvent = (level: LogLevel, event: string, fields?: LogFields, message?: string) => {
  writeEntry({
    ts: timestamp(),
    level,
    event,
    msg: message,
    fields,
  })
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
    logEvent('error', 'process.uncaught_exception', {
      error_name: error.name,
      error_message: error.message,
      stack: error.stack ?? null,
    })
  })
  process.on('unhandledRejection', (reason) => {
    logEvent('error', 'process.unhandled_rejection', { reason: sanitizeLogValue(reason) })
  })

  logEvent('info', 'logger.initialized', {
    path: ensureLogFilePath(),
    level: resolveMinimumLogLevel(),
  })
}

export const getMainProcessLogFilePath = () => ensureLogFilePath()
