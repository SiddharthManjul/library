# library-xtask

Purpose: repository tasks that must behave the same on Windows and Linux (run with `cargo xtask <task>`).

## Tasks
- `contracts [--check]` writes (or checks) `contracts/protocol.md` from the method table in `library-protocol`.
- `spec-split` copies README sections into `docs/spec/`; `spec-check` fails when a copy differs from the README.
- `schema-dump` writes `contracts/schema.sql` from the dev database (needs `pg_dump`).

## Rules
- No shell scripts for repo tasks; they would not run the same on Windows.
