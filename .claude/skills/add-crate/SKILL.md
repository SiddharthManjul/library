---
name: add-crate
description: Add a Rust crate with its AGENTS.md, a trait in core, tests and workspace wiring.
---

# Add crate

1. Create `crates/<name>/` with `Cargo.toml` (copy a sibling: `version.workspace`, `[lints] workspace = true`) and `src/lib.rs`.
2. Add `"crates/<name>"` to `members` in the root `Cargo.toml`.
3. Add `crates/<name>/AGENTS.md`: purpose, the rules from the root `AGENTS.md` that apply, commands, tests that must stay green.
4. Put the crate's public boundary as a trait in `library-core`; other crates depend on the trait, never on this crate's tables.
5. Every function that touches data takes a `RequestCtx`.
6. Write the first failing test, then implement.
7. `just check-fast`. Fix every clippy warning; never silence one to get green.
Do not read another crate's tables, and do not add a dependency from a leaf crate onto a peer.
