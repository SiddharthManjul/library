import { useMemo } from 'react'
import { Shell } from './components/Shell'
import { SignIn } from './components/SignIn'
import { StatusBar } from './components/StatusBar'
import { TopBar } from './components/TopBar'
import { useNode } from './node-session'

export function App() {
  const bridge = window.library
  const node = useNode(bridge)
  const signedIn = useMemo(() => bridge.previewShell || (node.phase === 'ready' && node.session.state === 'signed_in'), [bridge.previewShell, node])

  if (signedIn) return <Shell node={node} bridge={bridge} />
  return (
    <div className="shell shell-signin">
      <TopBar mode="workspace" onMode={() => undefined} layout={{ sidebar: { open: false, size: 0 }, panel: { open: false, size: 0, maximised: false }, agent: { open: false, size: 0 } }} onToggle={() => undefined} signedIn={false} modifier="Ctrl" bridge={bridge} />
      <SignIn node={node} />
      <StatusBar crumb={['Library', 'Sign in']} node={node} />
    </div>
  )
}
