import { BrowserWindow, dialog, ipcMain, shell } from 'electron'
import { writeFile, readFile, unlink } from 'fs/promises'
import { join, basename } from 'path'
import { tmpdir } from 'os'
import type { AutoLaunchSettings, CloseBehaviorPreference, CloseRequestAction } from '@/shared/window/bridge'

let registered = false
let setBackgroundSyncIntervalHandler: ((minutes: number) => void) | null = null
let setCloseBehaviorPreferenceHandler: ((value: CloseBehaviorPreference) => void) | null = null
let resolveCloseRequestHandler: ((window: BrowserWindow | null, action: CloseRequestAction, rememberBackground: boolean) => void) | null = null
let getAutoLaunchSettingsHandler: (() => Promise<AutoLaunchSettings> | AutoLaunchSettings) | null = null
let setAutoLaunchEnabledHandler: ((enabled: boolean) => Promise<AutoLaunchSettings> | AutoLaunchSettings) | null = null

const getWindowFromEvent = (event: Electron.IpcMainInvokeEvent) => BrowserWindow.fromWebContents(event.sender)

export const setWindowSyncIntervalHandler = (handler: (minutes: number) => void) => {
  setBackgroundSyncIntervalHandler = handler
}

export const setWindowCloseBehaviorHandler = (handler: (value: CloseBehaviorPreference) => void) => {
  setCloseBehaviorPreferenceHandler = handler
}

export const setWindowCloseResolveHandler = (
  handler: (window: BrowserWindow | null, action: CloseRequestAction, rememberBackground: boolean) => void,
) => {
  resolveCloseRequestHandler = handler
}

export const setWindowAutoLaunchHandlers = (
  getHandler: () => Promise<AutoLaunchSettings> | AutoLaunchSettings,
  setHandler: (enabled: boolean) => Promise<AutoLaunchSettings> | AutoLaunchSettings,
) => {
  getAutoLaunchSettingsHandler = getHandler
  setAutoLaunchEnabledHandler = setHandler
}

export const registerWindowIpc = () => {
  if (registered) {
    return
  }

  registered = true

  ipcMain.handle('window:minimize', (event) => {
    getWindowFromEvent(event)?.minimize()
  })

  ipcMain.handle('window:toggleMaximize', (event) => {
    const currentWindow = getWindowFromEvent(event)

    if (!currentWindow) {
      return false
    }

    if (currentWindow.isMaximized()) {
      currentWindow.unmaximize()
      return false
    }

    currentWindow.maximize()
    return true
  })

  ipcMain.handle('window:close', (event) => {
    getWindowFromEvent(event)?.close()
  })

  ipcMain.handle('window:isMaximized', (event) => getWindowFromEvent(event)?.isMaximized() ?? false)

  ipcMain.handle('window:openExternal', (_event, url: string) => {
    if (typeof url === 'string' && (url.startsWith('https://') || url.startsWith('http://'))) {
      return shell.openExternal(url)
    }
  })

  ipcMain.handle('window:getAutoLaunchSettings', async () => getAutoLaunchSettingsHandler?.() ?? { enabled: false, supported: false })

  ipcMain.handle('window:setAutoLaunchEnabled', async (_event, enabled: boolean) => {
    if (typeof enabled !== 'boolean') {
      return getAutoLaunchSettingsHandler?.() ?? { enabled: false, supported: false }
    }

    return setAutoLaunchEnabledHandler?.(enabled) ?? { enabled: false, supported: false }
  })

  ipcMain.handle('window:setCloseBehaviorPreference', (_event, value: CloseBehaviorPreference) => {
    if (value === 'ask' || value === 'always_background' || value === 'always_quit') {
      setCloseBehaviorPreferenceHandler?.(value)
    }
  })

  ipcMain.handle(
    'window:resolveCloseRequest',
    (event, action: CloseRequestAction, rememberBackground: boolean) => {
      if (action === 'background' || action === 'quit') {
        resolveCloseRequestHandler?.(
          getWindowFromEvent(event) ?? null,
          action,
          Boolean(rememberBackground),
        )
      }
    },
  )

  ipcMain.handle('window:setBackgroundSyncInterval', (_event, minutes: number) => {
    if (typeof minutes === 'number' && Number.isFinite(minutes) && minutes > 0) {
      setBackgroundSyncIntervalHandler?.(minutes)
    }
  })

  ipcMain.handle('window:exportPdf', async (_event, html: string, suggestedName: string) => {
    const safeName = (suggestedName || 'email').replace(/[/\\?%*:|"<>]/g, '_')
    const { canceled, filePath } = await dialog.showSaveDialog({
      defaultPath: `${safeName}.pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
    })
    if (canceled || !filePath) return false

    const tmpHtmlPath = join(tmpdir(), `mailyou-export-${Date.now()}.html`)
    await writeFile(tmpHtmlPath, html, 'utf-8')

    const win = new BrowserWindow({ show: false, width: 800, height: 600 })
    try {
      await win.loadFile(tmpHtmlPath)
      const pdfData = await win.webContents.printToPDF({
        printBackground: true,
      })
      await writeFile(filePath, pdfData)
      shell.showItemInFolder(filePath)
      return true
    } finally {
      win.close()
      unlink(tmpHtmlPath).catch(() => {})
    }
  })

  ipcMain.handle(
    'window:openTextFile',
    async (_event, filters: { name: string; extensions: string[] }[]) => {
      const { canceled, filePaths } = await dialog.showOpenDialog({
        filters,
        properties: ['openFile'],
      })
      if (canceled || filePaths.length === 0) return null
      const filePath = filePaths[0]
      const content = await readFile(filePath, 'utf-8')
      return { content, fileName: basename(filePath) }
    },
  )

  ipcMain.handle(
    'window:saveTextFile',
    async (
      _event,
      content: string,
      suggestedName: string,
      filters: { name: string; extensions: string[] }[],
    ) => {
      const { canceled, filePath } = await dialog.showSaveDialog({
        defaultPath: suggestedName,
        filters,
      })
      if (canceled || !filePath) return false
      await writeFile(filePath, content, 'utf-8')
      shell.showItemInFolder(filePath)
      return true
    },
  )

  ipcMain.handle(
    'window:saveBinaryFiles',
    async (
      _event,
      files: { fileName: string; mimeType: string; dataBase64: string }[],
      suggestedFolderName: string,
    ) => {
      if (!Array.isArray(files) || files.length === 0) {
        return false
      }

      const { canceled, filePaths } = await dialog.showOpenDialog({
        defaultPath: suggestedFolderName,
        properties: ['openDirectory', 'createDirectory'],
      })
      if (canceled || filePaths.length === 0) return false

      const targetDir = filePaths[0]
      for (const file of files) {
        const safeName = (file.fileName || 'attachment').replace(/[/\\?%*:|"<>]/g, '_')
        await writeFile(join(targetDir, safeName), Buffer.from(file.dataBase64, 'base64'))
      }
      shell.showItemInFolder(join(targetDir, files[0].fileName.replace(/[/\\?%*:|"<>]/g, '_')))
      return true
    },
  )
}
