<!-- Copy of README sections: "## Gateway and request path". Change the README and this file together; `cargo xtask spec-check` enforces it. -->

## Gateway and request path

Every call passes the same five checks in order, and the database checks scope a second time, so a bug in one layer cannot leak data.

```mermaid
%%{init: {"sequence": {"showSequenceNumbers": true, "mirrorActors": false, "messageAlign": "left"}}}%%
sequenceDiagram
  autonumber
  box rgb(219,234,254) Member side
    participant N as Local node
  end
  box rgb(254,243,199) Edge
    participant CF as Cloudflare Tunnel
  end
  box rgb(237,233,254) Server
    participant G as Gateway
    participant A as Authz
    participant H as Handler crate
    participant P as Postgres (FORCE RLS)
    participant T as Telemetry + audit
  end
  N->>CF: WSS JSON-RPC request (access token, idempotency key)
  CF->>G: forward (no inbound ports on the VPS)
  Note over G: Check 1: verify token signature + expiry + device
  G->>G: verify token
  Note over G: Check 2: rate limit per user and tenant, size limit
  G->>G: rate limit
  G->>A: Check 3: role + scope + plan required by method
  A-->>G: RequestCtx (tenant, principal, scopes, plan, taint, trace id)
  Note over G: Check 4: idempotency, a replay returns the first result
  G->>H: Check 5: call handler with RequestCtx
  H->>P: SET LOCAL app.tenant_id, app.user_id, app.scope_ids
  P-->>H: rows allowed by policy only
  H-->>G: typed result or typed error
  G->>T: span + audit row when state or side effect changed
  G-->>N: JSON-RPC response
```

- **Method declarations:** every method declares its role, scope and plan requirement in Rust; the gateway rejects before the handler runs.
- **Database role:** the runtime connects as `app_rw`, which is not the table owner and has no BYPASSRLS; every table uses FORCE ROW LEVEL SECURITY; policies check tenant and scope.
- **Errors:** typed (code, message, retryable); no stack traces or SQL leave the server.
- **Versioning:** the protocol is versioned (`v1`); incompatible app or node versions are refused at connect.
- **Push:** the gateway pushes notifications on the same socket (deltas, approvals, run updates; full list under API).
