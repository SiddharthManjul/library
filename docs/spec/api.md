<!-- Copy of README sections: "## API". Change the README and this file together; `cargo xtask spec-check` enforces it. -->

## API

Two links, both JSON-RPC 2.0: the desktop app talks to its local node over stdio, and the local node keeps one authenticated WebSocket to the gateway. HTTP is used only for sign-in and plugin OAuth callbacks and inbound webhooks.

### Local node to gateway

| Method | Who can call | Purpose |
| --- | --- | --- |
| `session.whoami` / `session.refresh` | Signed-in device | Tenant, roles, scopes, plan; token rotation |
| `sync.subscribe` | Any member, own scopes | Send each scope's applied_seq and state_root; receive missing deltas, then live ones |
| `sync.snapshot` | Any member, own scopes | Short-lived R2 link to the latest snapshot of a scope |
| `sync.key` | Any member, own scopes | Current scope key, wrapped to the device's public key |
| `org.departments.list` / `org.departments.upsert` | Admin | Departments, folds and members |
| `plugins.catalog` | Any member | Standard plugins with departments and sign-in mode |
| `connections.list` / `connections.start` / `connections.revoke` | Admin, department lead | List plugins; start a sign-in in any of the four modes; disconnect |
| `connections.oauth_client.set` | Admin | Store a customer-owned app's client id, secret and signing secret |
| `connections.setup_kit` | Admin | Slack manifest link or Google gcloud script for this tenant |
| `mcp.add` | Admin, department lead | Add any MCP server by URL; runs discovery and sign-in |
| `tools.pending` / `tools.review` | Admin | Review new or changed tool definitions; approve with a tier or block |
| `brain.status` / `brain.ask` | Product leader, C-suite | God's eye status and cited answers across their scopes |
| `brain.brief` | Product leader, C-suite, department lead | Latest precomputed brief for the caller or a department |
| `brain.context` | Harness on the server, own scopes | Cited slices for a run within a token budget |
| `brain.merges.pending` / `brain.merges.decide` | Department lead | Review entity merge proposals |
| `playbooks.list` / `playbooks.edit` / `playbooks.approve` | Department lead | Read, edit, pin, delete or approve playbook items |
| `skills.list` / `skills.upsert` | Admin | Built-in and tenant skill packs |
| `tasks.create` / `tasks.get` / `tasks.list` | Leaders create; members read own scopes | Cross-department tasks with items, progress and artifacts |
| `signoffs.pending` / `signoffs.decide` | Department lead | Approve or reject for their department |
| `runs.start` / `runs.get` / `runs.cancel` | Task owner; members for their own runs | Agent runs, steps, traces and budgets |
| `runs.lease` | Harness, own task | Acquire, renew or release an artifact lease |
| `model.call` | Harness in a local node | Model proxy with the tenant's key, budget-checked |
| `approvals.pending` / `approvals.decide` | Task owner, department lead | Approve each gated side effect from its preview |
| `actions.undo` | Whoever approved the action | Reverse an action where the plugin supports it |
| `proposals.list` / `proposals.decide` | Owner, department lead (higher tiers) | Act on ambient notifications, questions and proposals |
| `triggers.list` / `triggers.upsert` | Admin, department lead (higher tiers) | Ambient trigger rules |
| `audit.query` | Admin, C-suite | Read the tenant's audit log |
| `evals.results` | Admin | Latest eval runs per suite |
| `dispatch.call` | Harness in a local node; sandbox (after v0) | One tool call with step id and idempotency key |
| `jobs.claim` / `jobs.complete` (after v0) | Local node, own scopes | Volunteer compute |

**Pushed to the node:** `sync.delta`, `scope.revoked`, `key.rotated`, `task.updated`, `signoff.requested`, `approval.requested`, `run.updated`, `lease.changed`, `brief.ready`, `proposal.created`, `tool.changed`, `connection.expired`.

### Desktop app to local node

| Method | Purpose |
| --- | --- |
| `local.ask` | Answer from the replica (members' own scopes), offline-capable, with citations |
| `local.context` | Cited slices from the replica for a run started by this member |
| `local.sync_status` | Per-scope sequence, root check and lag, shown in the app |
| `local.runs` | Runs executing in this node, with steps and budgets |
| `remote.*` | Proxies every gateway method, so the app never touches tokens or the network |

**Rules:**

- Every method declares its role, scope and plan requirement in Rust; the gateway rejects before the handler runs, and row-level security rejects again in the database.
- Errors are typed (code, message, retryable); no stack traces or SQL leave the server.
- Writes carry an idempotency key; replays return the first result.
- The protocol is versioned (`v1`); incompatible app or node versions are refused at connect.
