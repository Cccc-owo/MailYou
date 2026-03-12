import { existsSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const currentDir = dirname(fileURLToPath(import.meta.url))
const repoRoot = process.cwd()
const manifestPath = resolve(repoRoot, 'src/rust/Cargo.toml')
const binaryName = process.platform === 'win32' ? 'mailstack-mail-backend.exe' : 'mailstack-mail-backend'
const bundledBinaryPath = resolve(currentDir, 'bin', binaryName)
const packagedBinaryPath = resolve(process.resourcesPath, 'bin', binaryName)
const releaseBinaryPath = resolve(repoRoot, 'src/rust/target/release', binaryName)

export interface RustSidecarLaunchSpec {
  command: string
  args: string[]
  description: string
}

export const getRustSidecarLaunchSpec = (): RustSidecarLaunchSpec => {
  if (process.env.VITE_DEV_SERVER_URL) {
    return {
      command: 'cargo',
      args: ['run', '--quiet', '--manifest-path', manifestPath],
      description: 'cargo-run',
    }
  }

  if (existsSync(packagedBinaryPath)) {
    return {
      command: packagedBinaryPath,
      args: [],
      description: 'packaged-binary',
    }
  }

  if (existsSync(bundledBinaryPath)) {
    return {
      command: bundledBinaryPath,
      args: [],
      description: 'bundled-binary',
    }
  }

  if (existsSync(releaseBinaryPath)) {
    return {
      command: releaseBinaryPath,
      args: [],
      description: 'release-binary',
    }
  }

  throw new Error(
    `Rust mail backend binary was not found. Checked ${packagedBinaryPath}, ${bundledBinaryPath}, and ${releaseBinaryPath}. Run "npm run build" or "npm run build:rust" before launching the desktop build.`,
  )
}
