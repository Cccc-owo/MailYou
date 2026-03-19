import { app, BrowserWindow, Menu, Notification, Tray, dialog, nativeImage, protocol, session } from 'electron'
import { existsSync, mkdirSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  getMailYouDevServerUrl,
  getMailYouOzonePlatformHint,
  isMailYouDevProtocolClientEnabled,
  isMailYouDevServerEnabled,
} from '@/config/runtime'
import { initializeMainProcessLogging } from './logging'
import { handleOAuthCallbackUrl } from './backend/oauth'
import { ensureRustBackendReady, onRustBackendEvent, shutdownRustBackend } from './backend/rust/process'
import { registerMailIpc } from './ipc/mail'
import { registerWindowIpc, setWindowAutoLaunchHandlers, setWindowCloseBehaviorHandler, setWindowCloseResolveHandler, setWindowSyncIntervalHandler } from './ipc/window'
import { mailBackend } from './backend/mailBackend'
import type { AutoLaunchSettings, CloseBehaviorPreference, CloseRequestAction } from '@/shared/window/bridge'

const __dirname = dirname(fileURLToPath(import.meta.url))
let mainWindow: BrowserWindow | null = null
let tray: Tray | null = null
let pendingOAuthCallbackUrl: string | null = null
let isQuitting = false
let isClosePromptVisible = false
let backgroundSyncIntervalMinutes = 5
let backgroundSyncTimer: ReturnType<typeof setInterval> | null = null
let backgroundSyncRunPromise: Promise<void> | null = null
const knownUnreadIdsByAccount = new Map<string, Set<string>>()
const isDev = isMailYouDevServerEnabled()
let closeBehaviorPreference: CloseBehaviorPreference = 'ask'
const AUTO_START_DESKTOP_FILE = 'MailYou.desktop'
const HIDDEN_START_ARG = '--hidden'

const MAIN_LOCALE_MESSAGES = {
  zh: {
    trayShow: '显示 MailYou',
    trayQuit: '退出',
  },
  en: {
    trayShow: 'Show MailYou',
    trayQuit: 'Quit',
  },
} as const

const getMainLocaleMessages = () => {
  const locale = app.getLocale().toLowerCase()
  return locale.startsWith('zh') ? MAIN_LOCALE_MESSAGES.zh : MAIN_LOCALE_MESSAGES.en
}

const isAutoLaunchSupported = () => !isDev && (process.platform === 'win32' || process.platform === 'linux')

const getAutoLaunchArgs = (hidden = false) => {
  if (process.defaultApp && process.argv.length >= 2) {
    return hidden ? [process.argv[1], HIDDEN_START_ARG] : [process.argv[1]]
  }

  return hidden ? [HIDDEN_START_ARG] : []
}

const escapeDesktopExecArg = (value: string) => value.replace(/(["\\`$])/g, '\\$1')

const getLinuxAutoStartExec = () => {
  const command = process.env.APPIMAGE || process.execPath
  const args = getAutoLaunchArgs(true)
  return [`"${escapeDesktopExecArg(command)}"`, ...args.map((arg) => `"${escapeDesktopExecArg(arg)}"`)].join(' ')
}

const getLinuxAutoStartFilePath = () => join(app.getPath('appData'), 'autostart', AUTO_START_DESKTOP_FILE)

const getAutoLaunchSettings = (): AutoLaunchSettings => {
  if (!isAutoLaunchSupported()) {
    return { enabled: false, supported: false }
  }

  if (process.platform === 'win32') {
    return {
      enabled: app.getLoginItemSettings({
        path: process.execPath,
        args: getAutoLaunchArgs(true),
      }).openAtLogin,
      supported: true,
    }
  }

  return {
    enabled: existsSync(getLinuxAutoStartFilePath()),
    supported: true,
  }
}

const setAutoLaunchEnabled = (enabled: boolean): AutoLaunchSettings => {
  if (!isAutoLaunchSupported()) {
    return { enabled: false, supported: false }
  }

  if (process.platform === 'win32') {
    app.setLoginItemSettings({
      openAtLogin: enabled,
      path: process.execPath,
      args: getAutoLaunchArgs(true),
    })
    return getAutoLaunchSettings()
  }

  const autostartFilePath = getLinuxAutoStartFilePath()

  if (!enabled) {
    rmSync(autostartFilePath, { force: true })
    return getAutoLaunchSettings()
  }

  mkdirSync(dirname(autostartFilePath), { recursive: true })
  writeFileSync(
    autostartFilePath,
    [
      '[Desktop Entry]',
      'Type=Application',
      'Version=1.0',
      'Name=MailYou',
      'Comment=Launch MailYou on login',
      `Exec=${getLinuxAutoStartExec()}`,
      'Terminal=false',
      'StartupNotify=false',
      'X-GNOME-Autostart-enabled=true',
    ].join('\n'),
    'utf-8',
  )

  return getAutoLaunchSettings()
}

const findBundledIconPath = (preferPng = false) => {
  const assetsDir = join(__dirname, '../assets')
  if (!existsSync(assetsDir)) {
    return null
  }

  const files = readdirSync(assetsDir)
  const pngLogo = files.find((file) => /^logo-.*\.png$/i.test(file))
  if (preferPng && pngLogo) {
    return join(assetsDir, pngLogo)
  }

  const svgLogo = files.find((file) => /^logo-.*\.svg$/i.test(file))
  if (svgLogo) {
    return join(assetsDir, svgLogo)
  }

  if (pngLogo) {
    return join(assetsDir, pngLogo)
  }

  return null
}

const resolveAppIconPath = () => {
  const candidates = [
    findBundledIconPath(),
    join(process.cwd(), 'src/assets/logo.svg'),
    join(__dirname, '../src/assets/logo.svg'),
    join(process.cwd(), 'src/assets/logo.png'),
    join(__dirname, '../src/assets/logo.png'),
  ].filter((candidate): candidate is string => Boolean(candidate))

  return candidates.find((candidate) => existsSync(candidate)) ?? candidates[0] ?? null
}

const resolveTrayIconPath = () => {
  const candidates = [
    join(process.cwd(), 'src/assets/logo.png'),
    join(__dirname, '../src/assets/logo.png'),
    findBundledIconPath(true),
    join(process.cwd(), 'src/assets/logo.svg'),
    join(__dirname, '../src/assets/logo.svg'),
  ].filter((candidate): candidate is string => Boolean(candidate))

  return candidates.find((candidate) => existsSync(candidate)) ?? candidates[0] ?? null
}

const createTrayIconImage = () => {
  const iconPath = resolveTrayIconPath()
  if (!iconPath) {
    return nativeImage.createEmpty()
  }

  const image = nativeImage.createFromPath(iconPath)
  if (image.isEmpty()) {
    return image
  }

  return image.resize({ width: 22, height: 22 })
}

const configureLinuxWindowSystem = () => {
  if (process.platform !== 'linux') {
    return
  }

  const ozoneHint = getMailYouOzonePlatformHint()

  app.commandLine.appendSwitch('enable-features', 'UseOzonePlatform')
  app.commandLine.appendSwitch('ozone-platform-hint', ozoneHint)
}

configureLinuxWindowSystem()
initializeMainProcessLogging()

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
const shouldLaunchHidden = process.argv.includes(HIDDEN_START_ARG) && !initialProtocolUrl

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
  const appIconPath = resolveAppIconPath() ?? undefined
  const window = new BrowserWindow({
    width: 1280,
    height: 820,
    minWidth: 1000,
    minHeight: 640,
    titleBarStyle: 'hidden',
    titleBarOverlay: false,
    backgroundColor: '#10131c',
    icon: appIconPath,
    show: !shouldLaunchHidden,
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
    void handleWindowCloseRequest(window)
  })

  const devServerUrl = getMailYouDevServerUrl()
  if (devServerUrl) {
    await window.loadURL(devServerUrl)
    if (!shouldLaunchHidden) {
      window.webContents.openDevTools({ mode: 'detach' })
    }
    mainWindow = window
    consumePendingOAuthCallback()
    return window
  }

  await window.loadFile(join(__dirname, '../index.html'))
  mainWindow = window
  consumePendingOAuthCallback()
  return window
}

const hideToBackground = (window: BrowserWindow) => {
  window.hide()
}

const handleWindowCloseRequest = async (window: BrowserWindow) => {
  if (closeBehaviorPreference === 'always_background') {
    hideToBackground(window)
    return
  }

  if (closeBehaviorPreference === 'always_quit') {
    isQuitting = true
    app.quit()
    return
  }

  if (isClosePromptVisible) {
    return
  }

  isClosePromptVisible = true
  if (!window.isDestroyed()) {
    window.webContents.send('window:closeRequested')
  }
}

const resolveWindowCloseRequest = (
  window: BrowserWindow | null,
  action: CloseRequestAction,
  rememberBackground: boolean,
) => {
  isClosePromptVisible = false
  const targetWindow = window && !window.isDestroyed() ? window : mainWindow
  if (!targetWindow || targetWindow.isDestroyed()) {
    return
  }

  if (action === 'background') {
    if (rememberBackground) {
      closeBehaviorPreference = 'always_background'
    }
    hideToBackground(targetWindow)
    return
  }

  isQuitting = true
  app.quit()
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

const unreadSetsEqual = (left: Set<string> | undefined, right: Set<string>) => {
  if (!left) {
    return false
  }

  if (left.size !== right.size) {
    return false
  }

  for (const value of left) {
    if (!right.has(value)) {
      return false
    }
  }

  return true
}

const publishUnreadChanges = (
  accountId: string,
  unreadMessages: Awaited<ReturnType<typeof mailBackend.getAccountUnreadSnapshot>>['unreadMessages'],
) => {
  const unreadIds = new Set(
    unreadMessages.map((message) => message.id),
  )
  const previousUnread = knownUnreadIdsByAccount.get(accountId)
  const newUnread = unreadMessages.filter(
    (message) => previousUnread && !previousUnread.has(message.id),
  )
  const unreadChanged = !unreadSetsEqual(previousUnread, unreadIds)

  knownUnreadIdsByAccount.set(accountId, unreadIds)

  if (unreadChanged) {
    sendBackgroundSyncComplete(accountId)
  }

  if (newUnread.length === 0 || !Notification.isSupported()) {
    return
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

const syncSingleAccountInBackground = async (accountId: string) => {
  const snapshot = await mailBackend.getAccountUnreadSnapshot(accountId)
  publishUnreadChanges(accountId, snapshot.unreadMessages)
}

const syncAccountsInBackground = async () => {
  if (backgroundSyncRunPromise) {
    return backgroundSyncRunPromise
  }

  backgroundSyncRunPromise = (async () => {
    try {
      const accounts = await mailBackend.listAccounts()
      for (const account of accounts) {
        await mailBackend.syncAccount(account.id)
        await syncSingleAccountInBackground(account.id)
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      console.error(`[background-sync] failed: ${message}`)
    }
  })().finally(() => {
    backgroundSyncRunPromise = null
  })

  return backgroundSyncRunPromise
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

  const messages = getMainLocaleMessages()
  const icon = createTrayIconImage()
  tray = new Tray(icon)
  tray.setToolTip('MailYou')
  tray.setContextMenu(Menu.buildFromTemplate([
    {
      label: messages.trayShow,
      click: () => {
        void focusMainWindow()
      },
    },
    {
      label: messages.trayQuit,
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

  onRustBackendEvent((event) => {
    if (event.event !== 'mailboxChanged') {
      return
    }

    void syncSingleAccountInBackground(event.payload.accountId).catch((error) => {
      const message = error instanceof Error ? error.message : String(error)
      console.error(`[realtime-sync] failed: ${message}`)
    })
  })

  registerMailIpc()
  registerWindowIpc()
  setWindowAutoLaunchHandlers(getAutoLaunchSettings, setAutoLaunchEnabled)
  setWindowCloseBehaviorHandler((value) => {
    closeBehaviorPreference = value
  })
  setWindowCloseResolveHandler(resolveWindowCloseRequest)
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
