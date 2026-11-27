---
name: add-migration
description: Add a Postgres table or change with forced row-level security, grants and isolation tests.
---

# Add migration

1. Create `migrations/<yyyymmddhhmmss>_<name>.sql`. Never edit an applied migration; add a new one (expand, backfill, switch, remove).
2. Add `tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE`, and `scope_id` when the data belongs to a department.
3. `ALTER TABLE t ENABLE ROW LEVEL SECURITY; ALTER TABLE t FORCE ROW LEVEL SECURITY;`
4. Add a policy using `app_tenant()`, `app_user()` and `app_scope_ids()` for both USING and WITH CHECK.
5. Grant only what is needed: `app_rw` for gateway work, `app_sequencer` for state tables (read-only to `app_rw`), `app_vault` for secrets, `app_auth` for sign-in.
6. The isolation suite (`crates/authz/tests/rls_isolation.rs`) seeds every table automatically. Run `just rls`. If the scoped table count changes, update the pinned number on purpose.
7. `just db-reset`, then `cargo xtask schema-dump` and `just check`. Update the README Data model section and run `just spec-split`.
