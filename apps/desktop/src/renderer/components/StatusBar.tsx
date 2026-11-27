import { Icon } from '../icons/Icon'
import type { NodeView } from '../node-session'

type Tone = 'good' | 'warn' | 'critical'

/** Status colour always comes with a word (BRAND.md section 3.4). */
function nodeLabel(v: NodeView): { text: string; tone: Tone } {
  if (v.phase === 'ready') return { text: `Local node ready · v${v.info.version}`, tone: 'good' }
  if (v.phase === 'connecting') return { text: 'Starting local node', tone: 'warn' }
  return { text: 'Local node unavailable', tone: 'critical' }
}

/** How the connection to Library's servers is doing, only once someone is signed in. */
function linkLabel(v: NodeView): { text: string; tone: Tone } | null {
  if (v.phase !== 'ready' || v.session.state !== 'signed_in') return null
  switch (v.session.link) {
    case 'online':
      return { text: 'Connected', tone: 'good' }
    case 'connecting':
      return { text: 'Connecting', tone: 'warn' }
    case 'offline':
      return { text: 'Offline, working from this computer', tone: 'warn' }
    case 'update_required':
      return { text: 'Update Library to reconnect', tone: 'critical' }
  }
}

export function StatusBar({ crumb, node }: { crumb: string[]; node: NodeView }) {
  const n = nodeLabel(node)
  const l = linkLabel(node)
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
        {l ? (
          <span className={`status status-${l.tone}`} role="status">
            <span className="dot" aria-hidden="true" />
            {l.text}
          </span>
        ) : null}
        <span className={`status status-${n.tone}`} role="status">
          <span className="dot" aria-hidden="true" />
          {n.text}
        </span>
      </div>
    </footer>
  )
}
