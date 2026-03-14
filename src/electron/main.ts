import { app, BrowserWindow, dialog, session } from 'electron'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { ensureRustBackendReady, shutdownRustBackend } from './backend/rust/process'
import { registerMailIpc } from './ipc/mail'
import { registerWindowIpc } from './ipc/window'

const __dirname = dirname(fileURLToPath(import.meta.url))

const configureLinuxWindowSystem = () => {
  if (process.platform !== 'linux') {
    return
  }

  const ozoneHint = process.env.MAILYOU_OZONE_PLATFORM_HINT?.trim() ?? 'auto'

  app.commandLine.appendSwitch('enable-features', 'UseOzonePlatform')
  app.commandLine.appendSwitch('ozone-platform-hint', ozoneHint)
}

configureLinuxWindowSystem()

const createMainWindow = async () => {
  const window = new BrowserWindow({
    width: 1280,
    height: 820,
    minWidth: 1000,
    minHeight: 640,
    titleBarStyle: 'hidden',
    titleBarOverlay: false,
    backgroundColor: '#10131c',
    icon: join(__dirname, '../src/assets/logo.png'),
    webPreferences: {
      preload: join(__dirname, 'preload.mjs'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  })

  if (process.env.VITE_DEV_SERVER_URL) {
    await window.loadURL(process.env.VITE_DEV_SERVER_URL)
    window.webContents.openDevTools({ mode: 'detach' })
    return window
  }

  await window.loadFile(join(__dirname, '../dist/index.html'))
  return window
}

app.whenReady().then(async () => {
  // Allow loading external images (http/https) and inline data URIs in email bodies.
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy': [
          "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' https: http: data: cid:; font-src 'self' data:; connect-src 'self' ws: wss: http: https:",
        ],
      },
    })
  })

  try {
    await ensureRustBackendReady()
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown Rust backend startup error'
    dialog.showErrorBox('MailYou backend failed to start', message)
    await shutdownRustBackend()
    app.quit()
    return
  }

  registerMailIpc()
  registerWindowIpc()
  await createMainWindow()

  app.on('activate', async () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      await createMainWindow()
    }
  })
})

app.on('before-quit', async () => {
  await shutdownRustBackend()
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})
