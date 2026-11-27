import { Icon } from '../icons/Icon'
import type { NodeView } from '../node-session'

/** Status colour always comes with a word (BRAND.md section 3.4). */
function nodeLabel(v: NodeView): { text: string; tone: 'good' | 'warn' | 'critical' } {
  if (v.phase === 'ready') return { text: `Local node ready · v${v.info.version}`, tone: 'good' }
  if (v.phase === 'connecting') return { text: 'Starting local node', tone: 'warn' }
  return { text: 'Local node unavailable', tone: 'critical' }
}

export function StatusBar({ crumb, node }: { crumb: string[]; node: NodeView }) {
  const n = nodeLabel(node)
  return (
    <footer className="statusbar">
      <div className="statusbar-left">
        <Icon name="terminal" size={16} />
        <Icon name="branch" size={16} />
        <Icon name="alert" size={16} />
        <span className="crumb">
          {crumb.map((c, i) => (
            <span key={c}>
              {i > 0 ? <span className="crumb-sep"> › </span> : null}
              {c}
            </span>
          ))}
        </span>
      </div>
      <div className="statusbar-right">
        <span className={`status status-${n.tone}`} role="status">
          <span className="dot" aria-hidden="true" />
          {n.text}
        </span>
      </div>
    </footer>
  )
}
