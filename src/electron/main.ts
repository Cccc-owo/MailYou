import { app, BrowserWindow, dialog, Menu, Notification, Tray, nativeImage, protocol, session } from 'electron'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  getMailYouDevServerUrl,
  getMailYouOzonePlatformHint,
  isMailYouDevProtocolClientEnabled,
  isMailYouDevServerEnabled,
} from '@/config/runtime'
import { handleOAuthCallbackUrl } from './backend/oauth'
import { ensureRustBackendReady, shutdownRustBackend } from './backend/rust/process'
import { registerMailIpc } from './ipc/mail'
import { registerWindowIpc, setWindowSyncIntervalHandler } from './ipc/window'
import { mailBackend } from './backend/mailBackend'

const __dirname = dirname(fileURLToPath(import.meta.url))
let mainWindow: BrowserWindow | null = null
let tray: Tray | null = null
let pendingOAuthCallbackUrl: string | null = null
let isQuitting = false
let backgroundSyncIntervalMinutes = 5
let backgroundSyncTimer: ReturnType<typeof setInterval> | null = null
const knownUnreadIdsByAccount = new Map<string, Set<string>>()
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

  window.on('close', (event) => {
    if (isQuitting) {
      return
    }

    event.preventDefault()
    window.hide()
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

const focusMainWindow = async () => {
  if (!mainWindow) {
    mainWindow = await createMainWindow()
    return
  }

  if (mainWindow.isMinimized()) {
    mainWindow.restore()
  }

  mainWindow.show()
  mainWindow.focus()
}

const sendBackgroundSyncComplete = (accountId: string) => {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send('mail:backgroundSyncComplete', accountId)
  }
}

const syncAccountsInBackground = async () => {
  try {
    const accounts = await mailBackend.listAccounts()
    for (const account of accounts) {
      await mailBackend.syncAccount(account.id)
      const bundle = await mailBackend.getMailboxBundle(account.id)
      const unreadIds = new Set(
        bundle.messages
          .filter((message) => !message.isRead)
          .map((message) => message.id),
      )
      const previousUnread = knownUnreadIdsByAccount.get(account.id)
      const newUnread = bundle.messages.filter(
        (message) => !message.isRead && previousUnread && !previousUnread.has(message.id),
      )

      knownUnreadIdsByAccount.set(account.id, unreadIds)
      sendBackgroundSyncComplete(account.id)

      if (newUnread.length === 0 || !Notification.isSupported()) {
        continue
      }

      const title = newUnread.length === 1 ? (newUnread[0].subject || 'New mail') : 'New mail'
      const body = newUnread.length === 1
        ? `From ${newUnread[0].from}`
        : `${newUnread.length} new unread messages`

      const notification = new Notification({ title, body, silent: false })
      notification.on('click', () => {
        void focusMainWindow()
      })
      notification.show()
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    console.error(`[background-sync] failed: ${message}`)
  }
}

const restartBackgroundSyncTimer = () => {
  if (backgroundSyncTimer) {
    clearInterval(backgroundSyncTimer)
    backgroundSyncTimer = null
  }

  backgroundSyncTimer = setInterval(() => {
    void syncAccountsInBackground()
  }, backgroundSyncIntervalMinutes * 60 * 1000)

  void syncAccountsInBackground()
}

const createTray = () => {
  if (tray) {
    return
  }

  const icon = nativeImage.createFromPath(join(__dirname, '../src/assets/logo.png'))
  tray = new Tray(icon)
  tray.setToolTip('MailYou')
  tray.setContextMenu(Menu.buildFromTemplate([
    {
      label: 'Show MailYou',
      click: () => {
        void focusMainWindow()
      },
    },
    {
      label: 'Quit',
      click: () => {
        isQuitting = true
        app.quit()
      },
    },
  ]))
  tray.on('click', () => {
    void focusMainWindow()
  })
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
  setWindowSyncIntervalHandler((minutes) => {
    backgroundSyncIntervalMinutes = minutes
    restartBackgroundSyncTimer()
  })

  protocol.handle('mailyou-avatar', async (request) => {
    const contactId = decodeURIComponent(request.url.slice('mailyou-avatar://'.length))
    const avatar = await mailBackend.getContactAvatar(contactId)
    if (!avatar) {
      return new Response('Not Found', { status: 404 })
    }

    const bytes = Buffer.from(avatar.dataBase64, 'base64')
    return new Response(bytes, {
      status: 200,
      headers: {
        'content-type': avatar.mimeType || 'application/octet-stream',
        'cache-control': 'no-store',
      },
    })
  })

  await createMainWindow()
  createTray()
  restartBackgroundSyncTimer()
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
  isQuitting = true
  if (backgroundSyncTimer) {
    clearInterval(backgroundSyncTimer)
    backgroundSyncTimer = null
  }
  await shutdownRustBackend()
})

app.on('window-all-closed', () => {
  if (isQuitting) {
    mainWindow = null
    if (process.platform !== 'darwin') {
      app.quit()
    }
  }
})
