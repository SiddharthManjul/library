import { Mark } from '../icons/Icon'
import type { NodeView } from '../node-session'

const PROVIDERS = ['Google', 'Microsoft', 'GitHub'] as const

export function SignIn({ node }: { node: NodeView }) {
  const unavailable = node.phase === 'unavailable'
  return (
    <main className="signin">
      <div className="signin-card pane">
        <span className="signin-mark">
          <Mark size={48} />
        </span>
        <h1 className="text-display signin-title">Sign in to Library</h1>
        <p className="signin-lead">Use the account your company set up.</p>
        <div className="signin-actions">
          {PROVIDERS.map((p) => (
            <button key={p} type="button" className="btn" disabled>
              Continue with {p}
            </button>
          ))}
        </div>
        {unavailable ? (
          <p className="signin-note signin-error" role="alert">
            The local node did not start, so signing in is not possible. Restart Library. If this keeps happening, reinstall the app.
          </p>
        ) : (
          <p className="signin-note">Sign-in is not connected in this build.</p>
        )}
      </div>
    </main>
  )
}
