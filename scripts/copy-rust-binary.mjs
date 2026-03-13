import { copyFileSync, existsSync, mkdirSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(scriptDir, '..')
const binaryName = process.platform === 'win32' ? 'mailyou-mail-backend.exe' : 'mailyou-mail-backend'
const sourcePath = resolve(repoRoot, 'src/rust/target/release', binaryName)
const targetDir = resolve(repoRoot, 'dist-electron/bin')
const targetPath = resolve(targetDir, binaryName)

if (!existsSync(sourcePath)) {
  throw new Error(`Rust mail backend binary was not found at ${sourcePath}. Run build:rust first.`)
}

mkdirSync(targetDir, { recursive: true })
copyFileSync(sourcePath, targetPath)

console.log(`Copied Rust mail backend to ${targetPath}`)
