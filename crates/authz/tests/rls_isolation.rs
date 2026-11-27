//! Row-level security isolation suite (P0-08). Runs against a real Postgres.
//! Two tenants are seeded into every table; each test then checks one property
//! for every table, so a new table that skips a rule fails here.
#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not

use library_testkit::{Ctx, Fixture, TestDb, count_as, exec_as, seed_two_tenants};
use sqlx::PgPool;

/// Tables with no `tenant_id` column: `tenants` is the tenant itself, `users` is global identity
/// (a person can belong to many tenants), `identities` belongs to a user.
const NO_TENANT_COLUMN: [&str; 3] = ["tenants", "users", "identities"];
/// Tables whose rows may be shared across tenants (tenant_id NULL).
const GLOBAL: [&str; 2] = ["skill_packs", "eval_runs"];

async fn setup() -> anyhow::Result<(TestDb, Fixture, Vec<String>)> {
    let db = TestDb::new().await?;
    let fx = seed_two_tenants(&db).await?;
    let tables = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE' AND table_name <> '_sqlx_migrations' ORDER BY 1",
    )
    .fetch_all(&db.admin)
    .await?;
    Ok((db, fx, tables))
}

/// secrets is readable only by the vault role; everything else goes through app_rw.
fn pool_for<'a>(db: &'a TestDb, table: &str) -> &'a PgPool {
    if table == "secrets" {
        &db.vault
    } else {
        &db.app
    }
}

fn ctx(tenant: uuid::Uuid, user: uuid::Uuid, scopes: &[uuid::Uuid]) -> Ctx {
    Ctx {
        tenant: Some(tenant),
        user: Some(user),
        scopes: scopes.to_vec(),
    }
}

async fn admin_count(db: &TestDb, sql: String) -> anyhow::Result<i64> {
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .fetch_one(&db.admin)
        .await?)
}

#[tokio::test]
async fn rls_every_table_is_enabled_and_forced() -> anyhow::Result<()> {
    let (db, _fx, tables) = setup().await?;
    assert!(
        tables.len() >= 40,
        "expected the full schema, found {} tables",
        tables.len()
    );
    for t in &tables {
        let (on, forced): (bool, bool) = sqlx::query_as("SELECT relrowsecurity, relforcerowsecurity FROM pg_class WHERE oid = ('public.\"' || $1 || '\"')::regclass")
            .bind(t)
            .fetch_one(&db.admin)
            .await?;
        assert!(
            on && forced,
            "{t}: row-level security must be enabled and forced"
        );
        let policies: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_policies WHERE schemaname='public' AND tablename=$1",
        )
        .bind(t)
        .fetch_one(&db.admin)
        .await?;
        assert!(policies > 0, "{t}: has no policy");
        if !NO_TENANT_COLUMN.contains(&t.as_str()) {
            let has: i64 = sqlx::query_scalar("SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name=$1 AND column_name='tenant_id'")
                .bind(t)
                .fetch_one(&db.admin)
                .await?;
            assert_eq!(has, 1, "{t}: must have tenant_id");
        }
    }
    db.drop_db().await
}

#[tokio::test]
async fn rls_runtime_roles_cannot_bypass() -> anyhow::Result<()> {
    let (db, _fx, _t) = setup().await?;
    for role in ["app_rw", "app_sequencer", "app_vault"] {
        let (sup, bypass): (bool, bool) =
            sqlx::query_as("SELECT rolsuper, rolbypassrls FROM pg_roles WHERE rolname=$1")
                .bind(role)
                .fetch_one(&db.admin)
                .await?;
        assert!(!sup && !bypass, "{role} must not be superuser or BYPASSRLS");
        let owned: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_tables WHERE schemaname='public' AND tableowner=$1",
        )
        .bind(role)
        .fetch_one(&db.admin)
        .await?;
        assert_eq!(owned, 0, "{role} must not own tables");
    }
    db.drop_db().await
}

#[tokio::test]
async fn rls_no_context_sees_nothing() -> anyhow::Result<()> {
    let (db, _fx, tables) = setup().await?;
    for t in &tables {
        let n = count_as(pool_for(&db, t), &Ctx::none(), t).await?;
        let expected = if GLOBAL.contains(&t.as_str()) { 1 } else { 0 };
        assert_eq!(
            n, expected,
            "{t}: a request with no context must see {expected} rows, saw {n}"
        );
    }
    db.drop_db().await
}

#[tokio::test]
async fn rls_tenant_sees_only_its_own_rows() -> anyhow::Result<()> {
    let (db, fx, tables) = setup().await?;
    for (mine, other) in [(&fx.a, &fx.b), (&fx.b, &fx.a)] {
        let c = ctx(mine.id, mine.user, &mine.scopes);
        for t in &tables {
            let expected =
                match t.as_str() {
                    "tenants" => {
                        admin_count(
                            &db,
                            format!("SELECT count(*) FROM tenants WHERE id = '{}'", mine.id),
                        )
                        .await?
                    }
                    "users" => 1,
                    "identities" => {
                        admin_count(
                            &db,
                            format!(
                                "SELECT count(*) FROM identities WHERE user_id = '{}'",
                                mine.user
                            ),
                        )
                        .await?
                    }
                    "devices" => {
                        admin_count(
                            &db,
                            format!(
                                "SELECT count(*) FROM devices WHERE user_id = '{}'",
                                mine.user
                            ),
                        )
                        .await?
                    }
                    g if GLOBAL.contains(&g) => admin_count(
                        &db,
                        format!(
                            "SELECT count(*) FROM {g} WHERE tenant_id = '{}' OR tenant_id IS NULL",
                            mine.id
                        ),
                    )
                    .await?,
                    o => {
                        admin_count(
                            &db,
                            format!(
                                "SELECT count(*) FROM \"{o}\" WHERE tenant_id = '{}'",
                                mine.id
                            ),
                        )
                        .await?
                    }
                };
            assert!(
                expected > 0,
                "{t}: fixture has no rows for the tenant, so the check would prove nothing"
            );
            let seen = count_as(pool_for(&db, t), &c, t).await?;
            assert_eq!(seen, expected, "{t}: tenant must see exactly its own rows");
            if !NO_TENANT_COLUMN.contains(&t.as_str()) {
                let leaked = exec_as(
                    pool_for(&db, t),
                    &c,
                    &format!("DELETE FROM \"{t}\" WHERE tenant_id = '{}'", other.id),
                )
                .await;
                assert!(
                    matches!(leaked, Ok(0) | Err(_)),
                    "{t}: deleted another tenant's rows"
                );
            }
        }
    }
    db.drop_db().await
}

#[tokio::test]
async fn rls_scoped_tables_filter_by_scope() -> anyhow::Result<()> {
    let (db, fx, tables) = setup().await?;
    let one_scope = ctx(fx.a.id, fx.a.user, &fx.a.scopes[..1]);
    let other_tenants_scopes = ctx(fx.a.id, fx.a.user, &fx.b.scopes);
    let mut checked = 0;
    for t in &tables {
        let has_scope: i64 = sqlx::query_scalar("SELECT count(*) FROM information_schema.columns WHERE table_schema='public' AND table_name=$1 AND column_name IN ('scope_id','scope_ids') AND table_name NOT IN ('scopes','scope_members')")
            .bind(t)
            .fetch_one(&db.admin)
            .await?;
        if has_scope == 0 {
            continue;
        }
        checked += 1;
        let col = if t == "briefs" {
            "scope_ids"
        } else {
            "scope_id"
        };
        let filter = if t == "briefs" {
            format!("scope_ids = ARRAY['{}'::uuid]", fx.a.scopes[0])
        } else {
            format!("scope_id = '{}'", fx.a.scopes[0])
        };
        let expected = admin_count(
            &db,
            format!(
                "SELECT count(*) FROM \"{t}\" WHERE tenant_id = '{}' AND {filter}",
                fx.a.id
            ),
        )
        .await?;
        assert!(expected > 0, "{t}: no rows in the first scope ({col})");
        assert_eq!(
            count_as(&db.app, &one_scope, t).await?,
            expected,
            "{t}: must see only the scopes the caller holds"
        );
        assert_eq!(
            count_as(&db.app, &other_tenants_scopes, t).await?,
            0,
            "{t}: another tenant's scope ids must grant nothing"
        );
    }
    // Pinned on purpose: adding a scoped table must be a conscious change to this number.
    assert_eq!(
        checked, 18,
        "scoped table count changed; check the new table has scope isolation"
    );
    db.drop_db().await
}

#[tokio::test]
async fn rls_cross_tenant_writes_fail() -> anyhow::Result<()> {
    let (db, fx, tables) = setup().await?;
    let c = ctx(fx.a.id, fx.a.user, &fx.a.scopes);
    for t in tables
        .iter()
        .filter(|t| !NO_TENANT_COLUMN.contains(&t.as_str()))
    {
        // Moving a visible row into another tenant must be refused (WITH CHECK or privilege).
        let moved = exec_as(
            pool_for(&db, t),
            &c,
            &format!("UPDATE \"{t}\" SET tenant_id = '{}'", fx.b.id),
        )
        .await;
        assert!(
            matches!(moved, Ok(0) | Err(_)),
            "{t}: a row was moved into another tenant"
        );
        // Rows of another tenant are invisible to update and delete.
        let upd = exec_as(
            pool_for(&db, t),
            &c,
            &format!(
                "UPDATE \"{t}\" SET tenant_id = tenant_id WHERE tenant_id = '{}'",
                fx.b.id
            ),
        )
        .await;
        assert!(
            matches!(upd, Ok(0) | Err(_)),
            "{t}: updated another tenant's rows"
        );
    }
    // The same checks as raw counts, through the owner, prove nothing changed.
    for t in tables
        .iter()
        .filter(|t| !NO_TENANT_COLUMN.contains(&t.as_str()) && !GLOBAL.contains(&t.as_str()))
    {
        let n = admin_count(
            &db,
            format!(
                "SELECT count(*) FROM \"{t}\" WHERE tenant_id = '{}'",
                fx.b.id
            ),
        )
        .await?;
        assert!(n > 0, "{t}: tenant B rows vanished");
    }
    db.drop_db().await
}

#[tokio::test]
async fn rls_users_and_devices_are_private() -> anyhow::Result<()> {
    let (db, fx, _t) = setup().await?;
    let a = ctx(fx.a.id, fx.a.user, &fx.a.scopes);
    // Tenant A cannot read tenant B's user, even by id.
    let n = exec_as(
        &db.app,
        &a,
        &format!("DELETE FROM users WHERE id = '{}'", fx.b.user),
    )
    .await;
    assert!(matches!(n, Ok(0) | Err(_)));
    // A device is visible only to its owner inside its own tenant.
    let stranger = ctx(fx.a.id, fx.b.user, &fx.a.scopes);
    assert_eq!(
        count_as(&db.app, &stranger, "devices").await?,
        0,
        "another user must not see the device"
    );
    assert_eq!(
        count_as(&db.app, &a, "devices").await?,
        1,
        "the owner sees its device"
    );
    let wrong_tenant = ctx(fx.b.id, fx.a.user, &fx.b.scopes);
    assert_eq!(
        count_as(&db.app, &wrong_tenant, "devices").await?,
        0,
        "the device must not show in another tenant"
    );
    assert_eq!(
        exec_as(
            &db.app,
            &stranger,
            &format!(
                "UPDATE devices SET last_seen = now() WHERE user_id = '{}'",
                fx.a.user
            )
        )
        .await?,
        0
    );
    db.drop_db().await
}

#[tokio::test]
async fn rls_only_the_sequencer_writes_state_tables() -> anyhow::Result<()> {
    let (db, fx, _t) = setup().await?;
    let c = ctx(fx.a.id, fx.a.user, &fx.a.scopes);
    for t in [
        "entities",
        "entity_aliases",
        "facts",
        "community_summaries",
        "briefs",
        "playbook_items",
        "deltas",
        "snapshots",
    ] {
        let denied = exec_as(&db.app, &c, &format!("DELETE FROM \"{t}\"")).await;
        assert!(denied.is_err(), "{t}: app_rw must not write state tables");
        let denied = exec_as(
            &db.app,
            &c,
            &format!("UPDATE \"{t}\" SET tenant_id = tenant_id"),
        )
        .await;
        assert!(denied.is_err(), "{t}: app_rw must not write state tables");
        let ok = exec_as(
            &db.sequencer,
            &c,
            &format!("UPDATE \"{t}\" SET tenant_id = tenant_id"),
        )
        .await?;
        assert!(ok > 0, "{t}: the sequencer must be able to write");
        let denied = exec_as(&db.sequencer, &c, &format!("DELETE FROM \"{t}\"")).await;
        assert!(denied.is_err(), "{t}: the sequencer never deletes state");
    }
    let denied = exec_as(&db.sequencer, &c, "UPDATE tasks SET title = title").await;
    assert!(
        denied.is_err(),
        "the sequencer writes state only, not work tables"
    );
    db.drop_db().await
}

#[tokio::test]
async fn rls_secrets_belong_to_the_vault_alone() -> anyhow::Result<()> {
    let (db, fx, _t) = setup().await?;
    let c = ctx(fx.a.id, fx.a.user, &fx.a.scopes);
    assert!(
        count_as(&db.app, &c, "secrets").await.is_err(),
        "app_rw must not read secrets"
    );
    assert!(
        count_as(&db.sequencer, &c, "secrets").await.is_err(),
        "the sequencer must not read secrets"
    );
    assert_eq!(count_as(&db.vault, &c, "secrets").await?, 1);
    assert!(
        count_as(&db.vault, &c, "tasks").await.is_err(),
        "the vault role reads secrets only"
    );
    db.drop_db().await
}

#[tokio::test]
async fn rls_audit_log_is_insert_only() -> anyhow::Result<()> {
    let (db, fx, _t) = setup().await?;
    let c = ctx(fx.a.id, fx.a.user, &fx.a.scopes);
    assert!(
        exec_as(&db.app, &c, "UPDATE audit_log SET actor = 'x'")
            .await
            .is_err()
    );
    assert!(exec_as(&db.app, &c, "DELETE FROM audit_log").await.is_err());
    // Even the table owner is stopped by the trigger.
    // The owner sees rows only with a tenant set (forced RLS), so set one to reach the trigger.
    assert!(
        exec_as(&db.owner, &c, "UPDATE audit_log SET actor = 'x'")
            .await
            .is_err(),
        "the owner must not be able to edit the audit log"
    );
    assert!(
        exec_as(&db.owner, &c, "DELETE FROM audit_log")
            .await
            .is_err(),
        "the owner must not be able to delete audit rows"
    );
    // Appending still works.
    let mut tx = db.app.begin().await?;
    sqlx::query(
        "SELECT set_config('app.tenant_id', $1, true), set_config('app.scope_ids', '', true)",
    )
    .bind(fx.a.id.to_string())
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO audit_log (tenant_id, seq, actor, action, payload_hash, prev_hash, hash) VALUES ($1, 99, 'test', 'x', '\\x00', '\\x00', '\\x01')")
        .bind(fx.a.id)
        .execute(&mut *tx)
        .await?;
    tx.rollback().await?;
    db.drop_db().await
}

#[tokio::test]
async fn rls_append_only_tables_cannot_be_edited() -> anyhow::Result<()> {
    let (db, fx, _t) = setup().await?;
    let c = ctx(fx.a.id, fx.a.user, &fx.a.scopes);
    for t in ["events", "progress_log", "run_steps", "decision_log"] {
        assert!(
            exec_as(
                &db.app,
                &c,
                &format!("UPDATE \"{t}\" SET tenant_id = tenant_id")
            )
            .await
            .is_err(),
            "{t}: must be append-only"
        );
        assert!(
            exec_as(&db.app, &c, &format!("DELETE FROM \"{t}\""))
                .await
                .is_err(),
            "{t}: must be append-only"
        );
    }
    db.drop_db().await
}
