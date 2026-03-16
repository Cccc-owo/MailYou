import { app, BrowserWindow, dialog, net, protocol, session } from 'electron'
import { dirname, join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import {
  getMailYouDevServerUrl,
  getMailYouOzonePlatformHint,
  isMailYouDevProtocolClientEnabled,
  isMailYouDevServerEnabled,
} from '@/config/runtime'
import { handleOAuthCallbackUrl } from './backend/oauth'
import { ensureRustBackendReady, shutdownRustBackend } from './backend/rust/process'
import { registerMailIpc } from './ipc/mail'
import { registerWindowIpc } from './ipc/window'
import { mailBackend } from './backend/mailBackend'

const __dirname = dirname(fileURLToPath(import.meta.url))
let mainWindow: BrowserWindow | null = null
let pendingOAuthCallbackUrl: string | null = null
const isDev = isMailYouDevServerEnabled()

const configureLinuxWindowSystem = () => {
  if (process.platform !== 'linux') {
    return
  }

  const ozoneHint = getMailYouOzonePlatformHint()

  app.commandLine.appendSwitch('enable-features', 'UseOzonePlatform')
  app.commandLine.appendSwitch('ozone-platform-hint', ozoneHint)
}

configureLinuxWindowSystem()

protocol.registerSchemesAsPrivileged([
  { scheme: 'mailyou-avatar', privileges: { secure: true, supportFetchAPI: true } },
])

const registerAppProtocolClient = () => {
  // Linux dev runs do not have a desktop file, so protocol registration via xdg-mime fails noisily.
  // Packaged builds still register normally, and dev can be opted in explicitly if needed.
  if (
    process.platform === 'linux' &&
    isDev &&
    !isMailYouDevProtocolClientEnabled()
  ) {
    return
  }

  if (process.defaultApp && process.argv.length >= 2) {
    app.setAsDefaultProtocolClient('mailyou', process.execPath, [process.argv[1]])
    return
  }

  app.setAsDefaultProtocolClient('mailyou')
}

const dispatchOAuthCallback = (rawUrl: string) => {
  if (!rawUrl.startsWith('mailyou://oauth/callback')) {
    return false
  }

  const handled = handleOAuthCallbackUrl(rawUrl)
  if (!handled) {
    pendingOAuthCallbackUrl = rawUrl
  }

  if (mainWindow) {
    if (mainWindow.isMinimized()) {
      mainWindow.restore()
    }
    mainWindow.focus()
  }

  return true
}

const consumePendingOAuthCallback = () => {
  if (!pendingOAuthCallbackUrl) {
    return
  }

  const rawUrl = pendingOAuthCallbackUrl
  pendingOAuthCallbackUrl = null
  dispatchOAuthCallback(rawUrl)
}

const tryExtractProtocolUrl = (argv: string[]) =>
  argv.find((value) => value.startsWith('mailyou://')) ?? null

const initialProtocolUrl = tryExtractProtocolUrl(process.argv)

if (!app.requestSingleInstanceLock()) {
  app.quit()
} else {
  app.on('second-instance', (_event, argv) => {
    const protocolUrl = tryExtractProtocolUrl(argv)
    if (protocolUrl) {
      dispatchOAuthCallback(protocolUrl)
    }
  })
}

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

  const devServerUrl = getMailYouDevServerUrl()
  if (devServerUrl) {
    await window.loadURL(devServerUrl)
    window.webContents.openDevTools({ mode: 'detach' })
    mainWindow = window
    consumePendingOAuthCallback()
    return window
  }

  await window.loadFile(join(__dirname, '../index.html'))
  mainWindow = window
  consumePendingOAuthCallback()
  return window
}

app.on('open-url', (event, url) => {
  event.preventDefault()
  dispatchOAuthCallback(url)
})

app.whenReady().then(async () => {
  registerAppProtocolClient()

  // Allow loading external images (http/https) and inline data URIs in email bodies.
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy': [
          "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' https: http: data: cid: mailyou-avatar:; font-src 'self' data:; connect-src 'self' ws: wss: http: https:",
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

  let storageDirCache: string | null = null
  protocol.handle('mailyou-avatar', async (request) => {
    if (!storageDirCache) {
      storageDirCache = await mailBackend.getStorageDir()
    }
    const relativePath = decodeURIComponent(request.url.slice('mailyou-avatar://'.length))
    const fullPath = join(storageDirCache, relativePath)
    return net.fetch(pathToFileURL(fullPath).toString())
  })

  await createMainWindow()
  if (initialProtocolUrl) {
    dispatchOAuthCallback(initialProtocolUrl)
  }

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
  mainWindow = null
  if (process.platform !== 'darwin') {
    app.quit()
  }
})
