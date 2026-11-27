import { useEffect, useReducer, useState } from 'react'
import { ActivityBar, ALL_VIEWS } from './ActivityBar'
import { EmptyState } from './EmptyState'
import { Resizer } from './Resizer'
import { StatusBar } from './StatusBar'
import { TopBar, type Mode } from './TopBar'
import type { NodeView } from '../node-session'
import type { LibraryBridge } from '../../shared/bridge'
import { initialLayout, layoutReducer } from '../state/layout'

function modifierFor(platform: string): string {
  return platform === 'darwin' ? '⌘' : 'Ctrl'
}

export function Shell({ node, bridge }: { node: NodeView; bridge: LibraryBridge }) {
  const platform = bridge.platform
  const [layout, dispatch] = useReducer(layoutReducer, initialLayout)
  const [mode, setMode] = useState<Mode>('workspace')
  const [view, setView] = useState('explorer')
  const modifier = modifierFor(platform)
  const current = ALL_VIEWS.find((v) => v.id === view) ?? ALL_VIEWS[0]!

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = platform === 'darwin' ? e.metaKey : e.ctrlKey
      if (!mod) return
      if (e.code === 'KeyB' && e.altKey) dispatch({ type: 'toggle', pane: 'agent' })
      else if (e.code === 'KeyB') dispatch({ type: 'toggle', pane: 'sidebar' })
      else if (e.code === 'KeyJ') dispatch({ type: 'toggle', pane: 'panel' })
      else return
      e.preventDefault()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [platform])

  const { sidebar, panel, agent } = layout
  return (
    <div className="shell">
      <TopBar mode={mode} onMode={setMode} layout={layout} onToggle={(pane) => dispatch({ type: 'toggle', pane })} signedIn modifier={modifier} bridge={bridge} />
      <div className="body">
        <ActivityBar active={view} onSelect={setView} />
        {sidebar.open ? (
          <>
            <aside className="sidebar pane" style={{ width: sidebar.size }} aria-label={`${current.label} sidebar`}>
              <h2 className="pane-title">{current.label.toUpperCase()}</h2>
              <EmptyState title="Nothing here yet">This view is not built yet.</EmptyState>
            </aside>
            <Resizer pane="sidebar" label="Resize sidebar" size={sidebar.size} axis="x" grow={1} onResize={(size) => dispatch({ type: 'resize', pane: 'sidebar', size })} />
          </>
        ) : null}
        <div className="center">
          {!(panel.open && panel.maximised) ? (
            <main className="workspace pane" aria-label={mode === 'agent' ? 'Agent' : 'Workspace'}>
              <EmptyState title={mode === 'agent' ? 'No agent running' : 'Nothing is open'}>
                {mode === 'agent' ? 'Agents show up here once a task starts.' : 'Open a file or start a task to begin.'}
              </EmptyState>
            </main>
          ) : null}
          {panel.open ? (
            <>
              {!panel.maximised ? <Resizer pane="panel" label="Resize panel" size={panel.size} axis="y" grow={-1} onResize={(size) => dispatch({ type: 'resize', pane: 'panel', size })} /> : null}
              <section className={`panel pane${panel.maximised ? ' panel-max' : ''}`} style={panel.maximised ? undefined : { height: panel.size }} aria-label="Panel">
                <div className="panel-head">
                  <h2 className="pane-title">TERMINAL</h2>
                  <button type="button" className="btn btn-quiet" onClick={() => dispatch({ type: panel.maximised ? 'restorePanel' : 'maximisePanel' })}>
                    {panel.maximised ? 'Restore panel' : 'Maximise panel'}
                  </button>
                </div>
                <EmptyState title="No terminal open" />
              </section>
            </>
          ) : null}
        </div>
        {agent.open ? (
          <>
            <Resizer pane="agent" label="Resize agent pane" size={agent.size} axis="x" grow={-1} onResize={(size) => dispatch({ type: 'resize', pane: 'agent', size })} />
            <aside className="agent pane" style={{ width: agent.size }} aria-label="Agent pane">
              <h2 className="pane-title">AGENT</h2>
              <EmptyState title="No agent running">Agents show up here once a task starts.</EmptyState>
            </aside>
          </>
        ) : null}
      </div>
      <StatusBar crumb={['Library', current.label]} node={node} />
    </div>
  )
}
