import { BrowserWindow, dialog, ipcMain, shell } from 'electron'
import { writeFile, unlink } from 'fs/promises'
import { join } from 'path'
import { tmpdir } from 'os'

let registered = false

const getWindowFromEvent = (event: Electron.IpcMainInvokeEvent) => BrowserWindow.fromWebContents(event.sender)

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

  ipcMain.handle('window:focus', (event) => {
    const win = getWindowFromEvent(event)
    if (win) {
      if (win.isMinimized()) win.restore()
      win.focus()
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
}
