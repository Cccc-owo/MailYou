import { BrowserWindow, ipcMain, shell } from 'electron'

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
}
