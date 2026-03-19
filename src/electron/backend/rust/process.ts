import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { createInterface, type Interface } from 'node:readline'
import type { RustBackendEvent, RustBackendMessage, RustBackendMethod, RustBackendMethodMap, RustBackendResponse } from './protocol'
import { getRustSidecarLaunchSpec } from './paths'
import { logEvent, type LogFields } from '../../logging'

const formatExitReason = (details: string, code: number | null, signal: NodeJS.Signals | null) =>
  details || `Rust mail backend exited (code: ${code ?? 'null'}, signal: ${signal ?? 'null'})`

const parseRustStderrLine = (line: string): { event: string, fields: LogFields } => {
  const match = line.match(/^\[([^\]]+)\]\s*(.*)$/)
  if (!match) {
    return {
      event: 'rust.stderr',
      fields: {
        line,
      },
    }
  }

  const [, component, message] = match
  return {
    event: `rust.${component}.stderr`,
    fields: {
      component,
      message,
    },
  }
}

class RustBackendClient {
  private process: ChildProcessWithoutNullStreams | null = null
  private stdoutReader: Interface | null = null
  private nextRequestId = 1
  private pending = new Map<
    number,
    {
      resolve: (value: unknown) => void
      reject: (reason: Error) => void
    }
  >()
  private startPromise: Promise<void> | null = null
  private lastStartError: Error | null = null
  private isShuttingDown = false
  private expectedExit = false
  private eventListeners = new Set<(event: RustBackendEvent) => void>()

  async invoke<M extends RustBackendMethod>(
    method: M,
    ...args: RustBackendMethodMap[M]['params'] extends undefined ? [] : [RustBackendMethodMap[M]['params']]
  ): Promise<RustBackendMethodMap[M]['result']> {
    await this.ensureStarted()

    if (this.isShuttingDown) {
      throw new Error('Rust mail backend is shutting down')
    }

    if (this.lastStartError) {
      throw this.lastStartError
    }

    const child = this.process
    if (!child || child.killed) {
      throw new Error('Rust mail backend is not running')
    }

    const id = this.nextRequestId++
    const payload = args[0] === undefined ? { id, method } : { id, method, params: args[0] }
    const startTime = Date.now()

    return await new Promise<RustBackendMethodMap[M]['result']>((resolve, reject) => {
      // Timeout: if the backend doesn't respond within 30s, reject the Promise.
      // syncAccount gets a longer timeout since IMAP operations can be slow.
      const timeoutMs = method === 'syncAccount' ? 120_000 : 30_000
      const timer = setTimeout(() => {
        if (this.pending.has(id)) {
          this.pending.delete(id)
          const elapsed = Date.now() - startTime
          logEvent('warn', 'rpc.timeout', { id, method, elapsed_ms: elapsed })
          reject(new Error(`Rust backend request '${method}' (id=${id}) timed out after ${timeoutMs / 1000}s`))
        }
      }, timeoutMs)

      this.pending.set(id, {
        resolve: (value: unknown) => {
          clearTimeout(timer)
          logEvent('info', 'rpc.response', {
            id,
            method,
            status: 'ok',
            elapsed_ms: Date.now() - startTime,
          })
          ;(resolve as (v: unknown) => void)(value)
        },
        reject: (reason: Error) => {
          clearTimeout(timer)
          logEvent('error', 'rpc.response', {
            id,
            method,
            status: 'error',
            elapsed_ms: Date.now() - startTime,
            error: reason.message,
          })
          reject(reason)
        },
      })

      logEvent('debug', 'rpc.request', { id, method })

      child.stdin.write(`${JSON.stringify(payload)}\n`, (error) => {
        if (!error) {
          return
        }

        clearTimeout(timer)
        this.pending.delete(id)
        logEvent('error', 'rpc.stdin_write_failed', {
          id,
          method,
          error: error.message,
        })
        reject(new Error(`Failed to send request to Rust mail backend: ${error.message}`))
      })
    })
  }

  async shutdown() {
    this.isShuttingDown = true
    this.lastStartError = null
    this.expectedExit = true

    if (this.stdoutReader) {
      this.stdoutReader.close()
      this.stdoutReader = null
    }

    if (!this.process) {
      this.isShuttingDown = false
      return
    }

    const child = this.process
    this.process = null

    if (!child.killed) {
      child.kill()
    }

    this.rejectPending(new Error('Rust mail backend was shut down'))
    this.isShuttingDown = false
  }

  onEvent(listener: (event: RustBackendEvent) => void) {
    this.eventListeners.add(listener)
    return () => {
      this.eventListeners.delete(listener)
    }
  }

  private async ensureStarted() {
    if (this.process && !this.process.killed) {
      return
    }

    if (this.isShuttingDown) {
      return
    }

    if (this.startPromise) {
      await this.startPromise
      return
    }

    this.startPromise = this.start()

    try {
      await this.startPromise
    } finally {
      this.startPromise = null
    }
  }

  private async start() {
    const launchSpec = getRustSidecarLaunchSpec()
    logEvent('info', 'rust.process_starting', {
      command: launchSpec.command,
      args: launchSpec.args,
      description: launchSpec.description,
    })
    const child = spawn(launchSpec.command, launchSpec.args, {
      stdio: 'pipe',
      env: {
        ...process.env,
        CARGO_TERM_COLOR: 'never',
      },
    })

    this.process = child
    this.lastStartError = null
    this.stdoutReader = createInterface({ input: child.stdout })
    this.stdoutReader.on('line', (line) => this.handleResponseLine(line))

    let recentStderr = ''
    child.stderr.on('data', (chunk) => {
      const text = chunk.toString('utf8')
      // Forward Rust backend stderr to console for visibility
      for (const line of text.split('\n')) {
        const trimmed = line.trim()
        if (!trimmed) {
          continue
        }

        const parsed = parseRustStderrLine(trimmed)
        const normalized = trimmed.toLowerCase()
        if (normalized.includes(' error') || normalized.startsWith('error') || normalized.includes(' panicked')) {
          logEvent('error', parsed.event, parsed.fields)
          continue
        }

        if (normalized.includes(' warn') || normalized.startsWith('warn')) {
          logEvent('warn', parsed.event, parsed.fields)
          continue
        }

        logEvent('info', parsed.event, parsed.fields)
      }
      recentStderr = `${recentStderr}${text}`.slice(-4000)
    })

    const handleProcessExit = (code: number | null, signal: NodeJS.Signals | null) => {
      this.process = null
      const wasExpected = this.expectedExit
      this.expectedExit = false

      if (wasExpected || this.isShuttingDown) {
        logEvent('info', 'rust.process_exit', {
          expected: true,
          code,
          signal: signal ?? null,
        })
        this.lastStartError = null
        return
      }

      const error = new Error(formatExitReason(recentStderr.trim(), code, signal))
      logEvent('error', 'rust.process_exit', {
        expected: false,
        code,
        signal: signal ?? null,
        error: error.message,
      })
      this.lastStartError = error
      this.rejectPending(error)
    }

    child.once('exit', handleProcessExit)

    await new Promise<void>((resolve, reject) => {
      let settled = false

      child.once('spawn', () => {
        settled = true
        logEvent('info', 'rust.process_started', {
          pid: child.pid ?? null,
        })
        resolve()
      })

      child.once('error', (error) => {
        if (settled) {
          return
        }

        settled = true
        this.process = null
        const startupError = new Error(`Failed to start Rust mail backend (${launchSpec.description}): ${error.message}`)
        logEvent('error', 'rust.process_start_failed', {
          description: launchSpec.description,
          error: startupError.message,
        })
        this.lastStartError = startupError
        reject(startupError)
      })

      child.once('exit', (code, signal) => {
        if (settled) {
          return
        }

        settled = true
        this.process = null
        const startupError = new Error(formatExitReason(recentStderr.trim(), code, signal))
        logEvent('error', 'rust.process_start_failed', {
          description: launchSpec.description,
          code,
          signal: signal ?? null,
          error: startupError.message,
        })
        this.lastStartError = startupError
        reject(startupError)
      })
    })
  }

  private handleResponseLine(line: string) {
    if (!line.trim()) {
      return
    }

    let message: RustBackendMessage

    try {
      message = JSON.parse(line) as RustBackendMessage
    } catch {
      logEvent('warn', 'rpc.non_json_line', { preview: line.slice(0, 200) })
      return
    }

    if ('event' in message) {
      for (const listener of this.eventListeners) {
        listener(message)
      }
      return
    }

    const response = message as RustBackendResponse
    const pending = this.pending.get(response.id)
    if (!pending) {
      return
    }

    this.pending.delete(response.id)

    if (response.ok) {
      pending.resolve(response.result)
      return
    }

    pending.reject(new Error(response.error.message))
  }

  private rejectPending(error: Error) {
    for (const pending of this.pending.values()) {
      pending.reject(error)
    }

    this.pending.clear()
  }
}

const rustBackendClient = new RustBackendClient()

export const invokeRustBackend = <M extends RustBackendMethod>(
  method: M,
  ...args: RustBackendMethodMap[M]['params'] extends undefined ? [] : [RustBackendMethodMap[M]['params']]
) => rustBackendClient.invoke(method, ...args)

export const ensureRustBackendReady = () => invokeRustBackend('healthCheck')

export const shutdownRustBackend = () => rustBackendClient.shutdown()

export const onRustBackendEvent = (listener: (event: RustBackendEvent) => void) =>
  rustBackendClient.onEvent(listener)
