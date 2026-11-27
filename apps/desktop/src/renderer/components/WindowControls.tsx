import type { LibraryBridge, WindowAction } from '../../shared/bridge'

const BUTTONS: { action: WindowAction; label: string; path: string }[] = [
  { action: 'minimize', label: 'Minimise', path: 'M5 12h14' },
  { action: 'maximize', label: 'Maximise', path: 'M6 6h12v12H6z' },
  { action: 'close', label: 'Close', path: 'M6 6l12 12M18 6 6 18' },
]

/** Window buttons for platforms where the app draws its own frame. Windows keeps its native ones. */
export function WindowControls({ bridge }: { bridge: LibraryBridge }) {
  if (bridge.platform !== 'linux') return null
  return (
    <div className="window-controls">
      {BUTTONS.map((b) => (
        <button key={b.action} type="button" className={`icon-btn window-btn window-${b.action}`} aria-label={b.label} onClick={() => void bridge.windowControl(b.action)}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
            <path d={b.path} />
          </svg>
        </button>
      ))}
    </div>
  )
}
