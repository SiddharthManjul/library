# Library IDE

Desktop app where every department works with its own AI agents on one shared company brain. Rust server (Postgres is the source of truth) plus a Rust local node and an Electron app on each member's machine. Windows 11 first, then Linux; macOS after v0. Full specification: README.md.

## Commands
- `just dev-up` starts Postgres in a container; `just db-reset` recreates the dev database and migrates.
- `just check-fast` runs fmt, clippy and tests; `just check` also runs the row-level security isolation tests.

## Rules
1. Crates talk only through traits in `library-core`; no crate reads another crate's tables.
2. Every call that touches data carries a `RequestCtx`.
3. Only the sequencer writes state tables. Replicas never write state.
4. Only the dispatcher calls plugins. Library credentials are decrypted only in the vault, dispatcher and model proxy.
5. Untrusted text reaches a model only through quarantine, and that call has no tools.
6. Every table has `tenant_id` with forced row-level security. The runtime role `app_rw` is not the owner and has no BYPASSRLS.
7. No secret, token, key or raw untrusted text in logs, errors, traces or test snapshots.
8. Everything keyed on official ids from the source system; aliases only propose merges.
9. Never weaken a test or lint to get green.
10. A coding agent acts outside its worktree only with its member's permission.
