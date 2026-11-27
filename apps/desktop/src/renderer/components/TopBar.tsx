import type { LibraryBridge } from '../../shared/bridge'
import { Icon, Mark, type IconName } from '../icons/Icon'
import { WindowControls } from './WindowControls'
import type { LayoutState, PaneId } from '../state/layout'

export type Mode = 'agent' | 'workspace'

interface Props {
  mode: Mode
  onMode(m: Mode): void
  layout: LayoutState
  onToggle(p: PaneId): void
  /** Shell controls (mode, layout) only show once a member is signed in. */
  signedIn: boolean
  modifier: string
  bridge: LibraryBridge
}

function Toggle({ icon, label, on, onClick, keys }: { icon: IconName; label: string; on: boolean; onClick(): void; keys: string }) {
  return (
    <button type="button" className="icon-btn" aria-label={label} aria-pressed={on} title={`${label} (${keys})`} onClick={onClick}>
      <Icon name={icon} />
    </button>
  )
}

export function TopBar({ mode, onMode, layout, onToggle, signedIn, modifier, bridge }: Props) {
  return (
    <header className="topbar">
      <div className="topbar-inner">
        <span className="topbar-mark" aria-label="Library">
          <Mark size={24} />
        </span>
        {signedIn ? (
          <>
            <div className="segmented" role="group" aria-label="View mode">
              {(['agent', 'workspace'] as const).map((m) => (
                <button key={m} type="button" className="segmented-btn" aria-pressed={mode === m} onClick={() => onMode(m)}>
                  {m === 'agent' ? 'Agent' : 'Workspace'}
                </button>
              ))}
            </div>
            <button type="button" className="dept-switch" aria-label="Switch department" aria-haspopup="menu" aria-expanded="false" disabled>
              <span className="dot" style={{ background: 'var(--dept-code)' }} aria-hidden="true" />
              Programming
              <Icon name="chevronDown" size={16} />
            </button>
            <button type="button" className="command" aria-label="Search or run a command" disabled>
              <Icon name="search" size={16} />
              <span>Search or run a command</span>
              <kbd>{modifier}+K</kbd>
            </button>
            <div className="topbar-right">
              <button type="button" className="icon-btn" aria-label="Inbox" disabled>
                <Icon name="bell" />
              </button>
              <button type="button" className="icon-btn" aria-label="More" disabled>
                <Icon name="more" />
              </button>
              <span className="topbar-divider" aria-hidden="true" />
              <Toggle icon="panelLeft" label="Toggle sidebar" on={layout.sidebar.open} onClick={() => onToggle('sidebar')} keys={`${modifier}+B`} />
              <Toggle icon="panelBottom" label="Toggle panel" on={layout.panel.open} onClick={() => onToggle('panel')} keys={`${modifier}+J`} />
              <Toggle icon="panelRight" label="Toggle agent pane" on={layout.agent.open} onClick={() => onToggle('agent')} keys={`${modifier}+Alt+B`} />
            </div>
          </>
        ) : null}
      </div>
      <WindowControls bridge={bridge} />
    </header>
  )
}
