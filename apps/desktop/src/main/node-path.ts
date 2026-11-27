import { join } from 'node:path'

export interface NodePathInput {
  isPackaged: boolean
  resourcesPath: string
  appPath: string
  platform: NodeJS.Platform
  override?: string
}

/** Where the local node binary lives: next to the app when installed, in the cargo target folder in development. */
export function resolveNodeBinary(i: NodePathInput): string {
  const exe = i.platform === 'win32' ? 'library-node.exe' : 'library-node'
  if (i.isPackaged) return join(i.resourcesPath, 'bin', exe)
  // Only development builds honour an override, so an installed app cannot be pointed at another binary.
  if (i.override) return i.override
  return join(i.appPath, '..', '..', 'target', 'debug', exe)
}
