import { mkdirSync, appendFileSync } from 'node:fs'
import { join } from 'node:path'
import { formatWithOptions } from 'node:util'

type ConsoleMethod = 'debug' | 'info' | 'log' | 'warn' | 'error'

const APP_DIR_NAME = 'MailYou'
const LOG_DIR_NAME = 'logs'

let initialized = false
let logFilePath: string | null = null

const originalConsole = {
  debug: console.debug.bind(console),
  info: console.info.bind(console),
  log: console.log.bind(console),
  warn: console.warn.bind(console),
  error: console.error.bind(console),
}

const timestamp = () => new Date().toISOString()

const currentLogFileName = () => `app-${timestamp().slice(0, 10)}.log`

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

const formatArgs = (args: unknown[]) =>
  formatWithOptions({ colors: false, depth: 6, breakLength: 120 }, ...args)

const writeLog = (method: ConsoleMethod, args: unknown[]) => {
  const message = formatArgs(args)
  const lines = message.split(/\r?\n/)
  const prefix = `[${timestamp()}]`
  const output = lines.map((line) => `${prefix} ${line}`).join('\n')

  try {
    appendFileSync(ensureLogFilePath(), `${output}\n`, 'utf8')
  } catch (error) {
    originalConsole.error(`[logger] failed to write log file: ${formatArgs([error])}`)
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

  console.log(`[logger] writing main-process logs to ${ensureLogFilePath()}`)
}

export const getMainProcessLogFilePath = () => ensureLogFilePath()
