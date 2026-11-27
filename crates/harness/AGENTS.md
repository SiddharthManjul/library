# library-harness

Purpose: Conduction, orchestration, synchronizer, evaluator and model router.
Spec: docs/spec (README section for this component).

## Rules
- Talk to other crates only through traits in `library-core`; never read another crate's tables.
- Every call that touches data carries a `RequestCtx`.
- No secret, token, key or raw untrusted text in logs, errors or test snapshots.

## Commands
- `just check-fast`
- `cargo nextest run -p library-harness`
