import { APP_METHODS } from '../generated/app-methods'

const allowed: ReadonlySet<string> = new Set(APP_METHODS)
const MAX_PARAMS_BYTES = 256 * 1024

export type Checked = { ok: true; method: string; params: Record<string, unknown> } | { ok: false; reason: string }

/** Everything the renderer sends to the node passes this check, in both the preload and the main process. */
export function checkInvoke(method: unknown, params: unknown): Checked {
  if (typeof method !== 'string' || !allowed.has(method)) return { ok: false, reason: 'unknown method' }
  const p = params ?? {}
  if (typeof p !== 'object' || Array.isArray(p) || Object.getPrototypeOf(p) !== Object.prototype) {
    return { ok: false, reason: 'params must be a plain object' }
  }
  let text: string
  try {
    text = JSON.stringify(p)
  } catch {
    return { ok: false, reason: 'params must be JSON' }
  }
  if (text.length > MAX_PARAMS_BYTES) return { ok: false, reason: 'params too large' }
  return { ok: true, method, params: p as Record<string, unknown> }
}

export const WINDOW_ACTIONS = ['minimize', 'maximize', 'close'] as const
export type WindowAction = (typeof WINDOW_ACTIONS)[number]

export function isWindowAction(a: unknown): a is WindowAction {
  return typeof a === 'string' && (WINDOW_ACTIONS as readonly string[]).includes(a)
}

export interface LibraryBridge {
  invoke(method: string, params?: Record<string, unknown>): Promise<unknown>
  onNodeStatus(cb: (status: string) => void): () => void
  /** Minimise, maximise or close the window. Only used where the app draws its own window buttons (Linux). */
  windowControl(action: WindowAction): Promise<void>
  platform: string
  /** Development only: show the shell without signing in. */
  previewShell: boolean
}
