import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { createInterface, type Interface } from 'node:readline'
import type { RustBackendMethod, RustBackendMethodMap, RustBackendResponse } from './protocol'
import { getRustSidecarLaunchSpec } from './paths'

const formatExitReason = (details: string, code: number | null, signal: NodeJS.Signals | null) =>
  details || `Rust mail backend exited (code: ${code ?? 'null'}, signal: ${signal ?? 'null'})`

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

    return await new Promise<RustBackendMethodMap[M]['result']>((resolve, reject) => {
      this.pending.set(id, { resolve: resolve as (value: unknown) => void, reject })

      child.stdin.write(`${JSON.stringify(payload)}\n`, (error) => {
        if (!error) {
          return
        }

        this.pending.delete(id)
        reject(new Error(`Failed to send request to Rust mail backend: ${error.message}`))
      })
    })
  }

  async shutdown() {
    this.isShuttingDown = true
    this.lastStartError = null

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
      recentStderr = `${recentStderr}${chunk.toString('utf8')}`.slice(-4000)
    })

    const handleProcessExit = (code: number | null, signal: NodeJS.Signals | null) => {
      const error = new Error(formatExitReason(recentStderr.trim(), code, signal))
      this.process = null
      this.lastStartError = error
      this.rejectPending(error)
    }

    child.once('exit', handleProcessExit)

    await new Promise<void>((resolve, reject) => {
      let settled = false

      child.once('spawn', () => {
        settled = true
        resolve()
      })

      child.once('error', (error) => {
        if (settled) {
          return
        }

        settled = true
        this.process = null
        const startupError = new Error(`Failed to start Rust mail backend (${launchSpec.description}): ${error.message}`)
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
        this.lastStartError = startupError
        reject(startupError)
      })
    })
  }

  private handleResponseLine(line: string) {
    if (!line.trim()) {
      return
    }

    let response: RustBackendResponse

    try {
      response = JSON.parse(line) as RustBackendResponse
    } catch {
      this.rejectPending(new Error(`Rust mail backend returned invalid JSON: ${line}`))
      return
    }

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
