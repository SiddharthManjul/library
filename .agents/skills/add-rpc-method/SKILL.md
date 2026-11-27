---
name: add-rpc-method
description: Add a JSON-RPC method with its role, scope and plan declaration, handler and rejection tests.
---

# Add rpc method

1. Add one `m(...)` line to `METHODS` in `crates/protocol/src/methods.rs`: name, link, access (role list), scope rule, lowest plan, write flag, purpose.
2. `just types` regenerates `contracts/protocol.md` and the app's TypeScript allowlist. Commit both.
3. The generated tests in `crates/protocol/tests/methods.rs` already cover role, plan and scope rejection for the new row. They must pass untouched.
4. Params and result types live in Rust with `#[derive(TS)]` and `#[ts(export, export_to = "protocol/")]`; the app never defines its own copy.
5. Writes require an idempotency key; a replay returns the first result.
6. Errors are `LibraryError` with a code from `ErrorCode`; no SQL, paths or stack traces in messages.
7. Update the README API section, then `just spec-split`.
