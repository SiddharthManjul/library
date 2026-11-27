import { spawn } from 'node:child_process'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { BrowserWindow, app, ipcMain, session, shell } from 'electron'
import tokens from '../../assets/brand/tokens.json'
import { checkInvoke, isWindowAction } from '../shared/bridge'
import { NodeClient, RpcFailure, type ChildLike } from './node-client'
import { resolveNodeBinary } from './node-path'
import { contentSecurityPolicy, isAppUrl, isSafeExternalUrl } from './security'

const isDev = !app.isPackaged
const devUrl = isDev ? process.env['ELECTRON_RENDERER_URL'] : undefined
const previewShell = isDev && process.env['LIBRARY_PREVIEW_SHELL'] === '1'

/** Window chrome colours come from the brand tokens (Ice Dark, the default theme). */
function brandColour(name: string): string {
  const t = tokens.color.tokens.find((x) => x.name === name)
  return (t?.value as Record<string, string> | undefined)?.['dark'] ?? '#000000'
}
const BAR_HEIGHT = Number.parseInt(tokens.layout.tokens.find((t) => t.name === 'bar-height')?.value ?? '44', 10)

const node = new NodeClient(() => {
  const bin = resolveNodeBinary({
    isPackaged: app.isPackaged,
    resourcesPath: process.resourcesPath,
    appPath: app.getAppPath(),
    platform: process.platform,
    override: process.env['LIBRARY_NODE_PATH'],
  })
  // Explicit argument list, no shell, and a minimal environment.
  return spawn(bin, [], { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true, env: { PATH: process.env['PATH'] ?? '' } }) as unknown as ChildLike
})

let win: BrowserWindow | null = null

function createWindow(): void {
  win = new BrowserWindow({
    width: 1440,
    height: 900,
    minWidth: 960,
    minHeight: 600,
    show: false,
    backgroundColor: brandColour('ground'),
    title: 'Library',
    icon: join(app.getAppPath(), 'build', 'icon.png'),
    // Windows keeps its native buttons (and Snap Layouts) drawn over our top bar. Linux window managers
    // differ too much for that, so the app draws its own buttons there.
    ...(process.platform === 'win32'
      ? { titleBarStyle: 'hidden' as const, titleBarOverlay: { color: brandColour('ground'), symbolColor: brandColour('ink'), height: BAR_HEIGHT } }
      : { frame: false }),
    webPreferences: {
      preload: join(__dirname, '../preload/index.cjs'),
      contextIsolation: true,
      sandbox: true,
      nodeIntegration: false,
      webSecurity: true,
      allowRunningInsecureContent: false,
      spellcheck: false,
      devTools: isDev,
      additionalArguments: previewShell ? ['--library-preview-shell'] : [],
    },
  })
  win.once('ready-to-show', () => win?.show())
  win.on('closed', () => (win = null))
  const target = devUrl ?? pathToFileURL(join(__dirname, '../renderer/index.html')).toString()
  void win.loadURL(target)
}

function harden(): void {
  const csp = contentSecurityPolicy(isDev, devUrl ?? undefined)
  session.defaultSession.webRequest.onHeadersReceived((details, cb) => {
    cb({ responseHeaders: { ...details.responseHeaders, 'Content-Security-Policy': [csp] } })
  })
  // No permission (camera, notifications, clipboard-read and so on) is ever granted to the renderer.
  session.defaultSession.setPermissionRequestHandler((_wc, _perm, cb) => cb(false))
  session.defaultSession.setPermissionCheckHandler(() => false)

  app.on('web-contents-created', (_e, contents) => {
    const appUrl = devUrl ?? pathToFileURL(join(__dirname, '../renderer/index.html')).toString()
    contents.setWindowOpenHandler(({ url }) => {
      if (isSafeExternalUrl(url)) void shell.openExternal(url)
      return { action: 'deny' }
    })
    contents.on('will-navigate', (event, url) => {
      if (!isAppUrl(url, appUrl)) {
        event.preventDefault()
        if (isSafeExternalUrl(url)) void shell.openExternal(url)
      }
    })
    contents.on('will-attach-webview', (event) => event.preventDefault())
  })
}

function wireIpc(): void {
  ipcMain.handle('library:invoke', async (event, method: unknown, params: unknown) => {
    // Only our own window's top frame may talk to the node.
    if (!win || event.sender !== win.webContents || event.senderFrame !== win.webContents.mainFrame) {
      throw new Error('Blocked: unknown sender.')
    }
    const checked = checkInvoke(method, params)
    if (!checked.ok) throw new Error(`Blocked: ${checked.reason}.`)
    try {
      return await node.request(checked.method, checked.params)
    } catch (e) {
      if (e instanceof RpcFailure) throw new Error(e.message, { cause: e })
      throw new Error('The local node is not available.', { cause: e })
    }
  })
  ipcMain.handle('library:window', (event, action: unknown) => {
    if (!win || event.sender !== win.webContents || event.senderFrame !== win.webContents.mainFrame) throw new Error('Blocked: unknown sender.')
    if (!isWindowAction(action)) throw new Error('Blocked: unknown window action.')
    if (action === 'minimize') win.minimize()
    else if (action === 'maximize') {
      if (win.isMaximized()) win.unmaximize()
      else win.maximize()
    }
    else win.close()
  })
  node.on('status', (status: string) => win?.webContents.send('library:node-status', status))
}

const single = app.requestSingleInstanceLock()
if (!single) {
  app.quit()
} else {
  app.on('second-instance', () => {
    if (win?.isMinimized()) win.restore()
    win?.focus()
  })
  void app.whenReady().then(() => {
    harden()
    wireIpc()
    node.start()
    createWindow()
    app.on('activate', () => BrowserWindow.getAllWindows().length === 0 && createWindow())
  })
  app.on('window-all-closed', () => app.quit())
  app.on('before-quit', () => node.stop())
}
