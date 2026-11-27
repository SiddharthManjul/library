import tokens from '../../../assets/brand/tokens.json'

export type PaneId = 'sidebar' | 'panel' | 'agent'

/** Sizes in px. Defaults come from the brand tokens; limits from BRAND.md section 5.1 (panel limits from the shell spec). */
export const LIMITS: Record<PaneId, { min: number; max: number }> = {
  sidebar: { min: 220, max: 420 },
  panel: { min: 100, max: 640 },
  agent: { min: 300, max: 520 },
}

/** Dragging this far below the minimum closes the pane, as in VS Code. */
export const CLOSE_BELOW_MIN = 40

function token(name: string, fallback: number): number {
  const v = tokens.layout.tokens.find((t) => t.name === name)?.value
  const n = v ? Number.parseInt(v, 10) : Number.NaN
  return Number.isFinite(n) ? n : fallback
}

export const DEFAULTS = {
  sidebar: token('sidebar-width', 264),
  panel: token('panel-height', 232),
  agent: token('agent-width', 340),
} as const

export interface LayoutState {
  sidebar: { open: boolean; size: number }
  panel: { open: boolean; size: number; maximised: boolean }
  agent: { open: boolean; size: number }
}

export const initialLayout: LayoutState = {
  sidebar: { open: true, size: DEFAULTS.sidebar },
  panel: { open: false, size: DEFAULTS.panel, maximised: false },
  agent: { open: true, size: DEFAULTS.agent },
}

export type LayoutAction =
  | { type: 'toggle'; pane: PaneId }
  | { type: 'resize'; pane: PaneId; size: number }
  | { type: 'maximisePanel' }
  | { type: 'restorePanel' }

const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n))

export function layoutReducer(state: LayoutState, action: LayoutAction): LayoutState {
  switch (action.type) {
    case 'toggle': {
      const p = state[action.pane]
      const next = { ...p, open: !p.open }
      if (action.pane === 'panel' && !next.open) (next as LayoutState['panel']).maximised = false
      return { ...state, [action.pane]: next }
    }
    case 'resize': {
      const { min, max } = LIMITS[action.pane]
      const p = state[action.pane]
      // Past the minimum by enough: close, but remember the last good size for reopening.
      if (action.size < min - CLOSE_BELOW_MIN) {
        const closed = { ...p, open: false }
        if (action.pane === 'panel') (closed as LayoutState['panel']).maximised = false
        return { ...state, [action.pane]: closed }
      }
      return { ...state, [action.pane]: { ...p, open: true, size: clamp(action.size, min, max) } }
    }
    case 'maximisePanel':
      return { ...state, panel: { ...state.panel, open: true, maximised: true } }
    case 'restorePanel':
      return { ...state, panel: { ...state.panel, maximised: false } }
  }
}
