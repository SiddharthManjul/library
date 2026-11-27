# Progress log

Append only. Newest entries at the bottom. Format: time, task, who, what.

## 2026-11-26 · P0 (first pass) · Claude Opus 5.5
Plan: spec cross-check, repo skeleton, database schema with row-level security, isolation tests.
Did: README and BUILD_GUIDE updated for the brand kit, Windows-first platforms, no fork, coding-agent permissions; cargo workspace; 44-table schema with forced RLS; testkit; 11 isolation tests; library-core.
Tests: 16 passing at the end of the pass; isolation suite mutation-checked.
Next: session tokens and device keys.

## 2026-11-26 · P0-10, P0-11 · Claude Opus 5.5
Did: `tenant_id` added to `devices`; `identities` and `refresh_tokens` tables and the `app_auth` role; Ed25519 access tokens; rotating refresh tokens with reuse detection; device keys in the OS keychain with a registration proof.
Tests: 41 passing. The Linux Secret Service test passes against a private keyring; Windows Credential Manager runs in CI.
Surprises: sqlx 0.9 needs `AssertSqlSafe` for dynamic SQL; JSON Schema `format: uuid` is not enforced by validators, so the schemas use patterns.
Next: the rest of P0.

## 2026-11-26 · P0-02 to P0-06, P0-12 to P0-15 · Claude Opus 5.5
Did: protocol crate (68-method table with role, scope and plan checks, generated tests per method); contracts (protocol.md, schema.sql, four JSON schemas with fixtures, replica schema); TypeScript types through ts-rs; xtask for contracts and the README spec split; ten skills; twelve ADRs; node stdio JSON-RPC server; Electron app written directly on the brand tokens (no fork); Linux window controls; deb and AppImage packaging with Electron fuses; Store tiles; CI and release workflows; cargo-deny policy; `chrono` replaced by `time` after an unmaintained-crate advisory.
Tests: 74 Rust tests (1 skipped: OS keychain), 40 desktop unit tests, 7 end-to-end tests against the real app and node, 2 against the unpacked package. Negative controls: removing the bundled node fails the packaged test; an unhardened Electron opens the inspector and the hardened one does not.
Surprises: electron-builder 26 has an `appx` target, not `msix` (same family, the Store accepts it); pnpm 12 refuses packages less than a day old and blocks install scripts unless listed in `allowBuilds`; the hardened Electron ignores Playwright's `--inspect`, so the packaged test attaches over CDP.
Open: P0-06 needs a first green run on GitHub; P0-15 needs the appx built and installed on Windows 11; P0-16 (OAuth providers) deferred by the founder. P0-10's disk encryption check for Windows (Section 9 item 10) is still to be researched before P4-04.
Next: P1 Trust core (gateway, vault, audit, tracing), after the two pending P0 checks.
