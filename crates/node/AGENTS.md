# library-node

Purpose: Local node: session, sync, replica, retrieval, provider runners, relay worker, harness.

## Rules
- Wiring only; logic lives in the component crates.
- Never log secrets, tokens or raw untrusted text.

## Commands
- `just check-fast`
- `cargo nextest run -p library-node`
