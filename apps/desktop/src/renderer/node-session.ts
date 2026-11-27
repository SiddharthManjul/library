import { useEffect, useState } from 'react'
import type { NodeInfo } from '../generated/protocol/NodeInfo'
import type { SessionState } from '../generated/protocol/SessionState'
import type { LibraryBridge } from '../shared/bridge'

declare global {
  interface Window {
    library: LibraryBridge
  }
}

export type NodeView =
  | { phase: 'connecting' }
  | { phase: 'ready'; info: NodeInfo; session: SessionState }
  | { phase: 'unavailable' }

/** Asks the local node who it is and whether a member is signed in, again whenever the node becomes ready. */
export function useNode(bridge: LibraryBridge): NodeView {
  const [view, setView] = useState<NodeView>({ phase: 'connecting' })
  useEffect(() => {
    let live = true
    const load = async () => {
      try {
        const [info, session] = await Promise.all([bridge.invoke('node.info'), bridge.invoke('session.state')])
        if (live) setView({ phase: 'ready', info: info as NodeInfo, session: session as SessionState })
      } catch {
        if (live) setView({ phase: 'unavailable' })
      }
    }
    void load()
    const off = bridge.onNodeStatus((s) => {
      if (s === 'ready') void load()
      else if (s === 'failed') setView({ phase: 'unavailable' })
      else if (s === 'restarting') setView({ phase: 'connecting' })
    })
    return () => {
      live = false
      off()
    }
  }, [bridge])
  return view
}
