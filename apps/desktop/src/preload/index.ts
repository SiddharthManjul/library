import { contextBridge, ipcRenderer } from 'electron'
import { checkInvoke, isWindowAction, type LibraryBridge } from '../shared/bridge'

// The renderer gets exactly this object: no ipcRenderer, no Node, no tokens.
const bridge: LibraryBridge = {
  invoke(method, params) {
    const checked = checkInvoke(method, params)
    if (!checked.ok) return Promise.reject(new Error(`Blocked: ${checked.reason}.`))
    return ipcRenderer.invoke('library:invoke', checked.method, checked.params)
  },
  onNodeStatus(cb) {
    const listener = (_e: unknown, status: string) => cb(status)
    ipcRenderer.on('library:node-status', listener)
    return () => ipcRenderer.removeListener('library:node-status', listener)
  },
  windowControl(action) {
    if (!isWindowAction(action)) return Promise.reject(new Error('Blocked: unknown window action.'))
    return ipcRenderer.invoke('library:window', action)
  },
  platform: process.platform,
  previewShell: process.argv.includes('--library-preview-shell'),
}

contextBridge.exposeInMainWorld('library', bridge)
