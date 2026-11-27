# 0002 Tool and dependency versions

Status: accepted
Date: 2026-11-26

## Context
Builds must be repeatable on Windows and Linux, and agents must not drift onto different toolchains.

## Decision
Rust 1.100.0 (edition 2024, pinned in `rust-toolchain.toml`), Node 26 with pnpm 12 for the desktop app, Postgres 16 for the server and tests, sqlx 0.9, ed25519-dalek 3, x25519-dalek 3, keyring 4, ts-rs 12, jsonschema 0.63. Lockfiles are committed. Dependencies upgrade one family per pull request.

## Options considered
Floating versions (breaks reproducibility). Nix (strong pinning, poor Windows support).

## Consequences
A new dependency family needs a line here. The toolchain moves only deliberately.
