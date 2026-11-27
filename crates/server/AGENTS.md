# library-server

Purpose: Server binary: wires every crate into one process.

## Rules
- Wiring only; logic lives in the component crates.
- Never log secrets, tokens or raw untrusted text.

## Commands
- `just check-fast`
- `cargo nextest run -p library-server`
