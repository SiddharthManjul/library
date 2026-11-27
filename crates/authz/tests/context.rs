#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not
//! Who can read what (README: Access scopes), checked against a realistic company and a real database.

use std::collections::{BTreeMap, BTreeSet};

use library_authz::{begin, build_ctx};
use library_core::{DeviceId, ErrorCode, Plan, Role, ScopeId, TenantId, UserId};
use library_testkit::TestDb;
use uuid::Uuid;

/// A person in the fixture: name, membership role, and (scope, scope role) pairs.
type Person = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

struct Org {
    db: TestDb,
    tenant: Uuid,
    scope: BTreeMap<&'static str, Uuid>,
    user: BTreeMap<&'static str, (Uuid, Uuid)>, // name -> (user id, device id)
}

async fn exec(db: &TestDb, sql: &str, binds: &[&Uuid]) {
    let mut q = sqlx::query(sqlx::AssertSqlSafe(sql.to_string()));
    for b in binds {
        q = q.bind(**b);
    }
    q.execute(&db.admin).await.unwrap();
}

/// Product fold F = programming + design + marketing. Operations stands alone. Finance and legal are sealed.
async fn org() -> Org {
    let db = TestDb::new().await.unwrap();
    let tenant = Uuid::new_v4();
    exec(
        &db,
        "INSERT INTO tenants (id, name) VALUES ($1, 'acme')",
        &[&tenant],
    )
    .await;
    exec(
        &db,
        "INSERT INTO plans (tenant_id, plan) VALUES ($1, 'business')",
        &[&tenant],
    )
    .await;

    let mut scope = BTreeMap::new();
    let fold = Uuid::new_v4();
    scope.insert("fold", fold);
    exec(
        &db,
        "INSERT INTO scopes (id, tenant_id, kind, name) VALUES ($1, $2, 'fold', 'product')",
        &[&fold, &tenant],
    )
    .await;
    for (name, in_fold, sealed) in [
        ("programming", true, false),
        ("design", true, false),
        ("marketing", true, false),
        ("operations", false, false),
        ("finance", false, true),
        ("legal", false, true),
    ] {
        let id = Uuid::new_v4();
        scope.insert(name, id);
        let sql = format!(
            "INSERT INTO scopes (id, tenant_id, kind, name, sealed, fold_id) VALUES ($1, $2, 'department', '{name}', {sealed}, {})",
            if in_fold { "$3" } else { "NULL" }
        );
        if in_fold {
            exec(&db, &sql, &[&id, &tenant, &fold]).await;
        } else {
            exec(&db, &sql, &[&id, &tenant]).await;
        }
    }

    // (name, membership role, [(scope, scope role)])
    let people: [Person; 7] = [
        ("dev", "member", &[("programming", "member")]),
        ("finance_clerk", "member", &[("finance", "member")]),
        ("product_leader", "product_leader", &[]),
        ("ceo", "c_suite", &[("legal", "member")]),
        (
            "designer_in_finance",
            "member",
            &[("design", "member"), ("finance", "member")],
        ),
        ("design_lead", "member", &[("design", "lead")]),
        ("ops", "member", &[("operations", "member")]),
    ];
    let mut user = BTreeMap::new();
    for (name, role, scopes) in people {
        let (u, d) = (Uuid::new_v4(), Uuid::new_v4());
        exec(&db, &format!("INSERT INTO users (id, email, display_name) VALUES ($1, '{name}@acme.test', '{name}')"), &[&u]).await;
        exec(
            &db,
            &format!(
                "INSERT INTO memberships (tenant_id, user_id, role) VALUES ($1, $2, '{role}')"
            ),
            &[&tenant, &u],
        )
        .await;
        exec(&db, &format!("INSERT INTO devices (id, tenant_id, user_id, public_key, signing_public_key) VALUES ($1, $2, $3, decode(md5('{name}k'),'hex'), decode(md5('{name}s'),'hex'))"), &[&d, &tenant, &u]).await;
        for (s, r) in scopes {
            exec(&db, &format!("INSERT INTO scope_members (tenant_id, scope_id, user_id, scope_role) VALUES ($1, $2, $3, '{r}')"), &[&tenant, &scope[s], &u]).await;
        }
        user.insert(name, (u, d));
    }
    Org {
        db,
        tenant,
        scope,
        user,
    }
}

impl Org {
    async fn can_read(&self, who: &str) -> BTreeSet<&'static str> {
        let (u, d) = self.user[who];
        let ctx = build_ctx(&self.db.auth, UserId(u), TenantId(self.tenant), DeviceId(d))
            .await
            .unwrap();
        self.scope
            .iter()
            .filter(|(_, id)| ctx.can_read(ScopeId(**id)))
            .map(|(n, _)| *n)
            .collect()
    }
}

fn set(names: &[&'static str]) -> BTreeSet<&'static str> {
    names.iter().copied().collect()
}

#[tokio::test]
async fn a_member_of_one_team_reads_their_whole_fold_and_nothing_sealed() {
    let o = org().await;
    assert_eq!(
        o.can_read("dev").await,
        set(&["programming", "design", "marketing", "fold"])
    );
    assert_eq!(
        o.can_read("ops").await,
        set(&["operations"]),
        "operations is not in the product fold"
    );
    o.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn sealed_scopes_are_readable_only_by_their_own_members() {
    let o = org().await;
    assert_eq!(o.can_read("finance_clerk").await, set(&["finance"]));
    // Belonging to finance does not widen into the fold, and belonging to the fold does not open finance.
    assert_eq!(
        o.can_read("designer_in_finance").await,
        set(&["programming", "design", "marketing", "fold", "finance"])
    );
    assert!(!o.can_read("dev").await.contains("finance"));
    assert!(!o.can_read("dev").await.contains("legal"));
    o.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn leaders_read_every_unsealed_scope_but_not_sealed_ones_they_do_not_belong_to() {
    let o = org().await;
    assert_eq!(
        o.can_read("product_leader").await,
        set(&["programming", "design", "marketing", "operations", "fold"])
    );
    // God's eye shows sealed scopes as summaries; the records stay sealed even for the C-suite.
    assert_eq!(
        o.can_read("ceo").await,
        set(&[
            "programming",
            "design",
            "marketing",
            "operations",
            "fold",
            "legal"
        ])
    );
    assert!(!o.can_read("ceo").await.contains("finance"));
    o.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn roles_and_plan_come_from_the_database() {
    let o = org().await;
    let ctx = |who: &str| {
        let (u, d) = o.user[who];
        build_ctx(&o.db.auth, UserId(u), TenantId(o.tenant), DeviceId(d))
    };
    let lead = ctx("design_lead").await.unwrap();
    assert!(
        lead.has_role(Role::Member) && lead.has_role(Role::DepartmentLead),
        "a scope lead is a department lead"
    );
    assert!(!ctx("dev").await.unwrap().has_role(Role::DepartmentLead));
    assert!(ctx("ceo").await.unwrap().has_role(Role::CSuite));
    assert_eq!(ctx("dev").await.unwrap().plan, Plan::Business);
    sqlx::query("DELETE FROM plans")
        .execute(&o.db.admin)
        .await
        .unwrap();
    assert_eq!(
        ctx("dev").await.unwrap().plan,
        Plan::Starter,
        "no plan row means the free plan"
    );
    o.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn no_context_for_a_revoked_device_a_stranger_or_a_removed_member() {
    let o = org().await;
    let (u, d) = o.user["dev"];
    let (other_user, other_dev) = o.user["ops"];
    let t = TenantId(o.tenant);
    let fail = |r: Result<_, library_core::LibraryError>| {
        r.map(|_: library_core::RequestCtx| ()).unwrap_err().code
    };
    // Someone else's device.
    assert_eq!(
        fail(build_ctx(&o.db.auth, UserId(u), t, DeviceId(other_dev)).await),
        ErrorCode::Unauthenticated
    );
    // Another tenant.
    assert_eq!(
        fail(build_ctx(&o.db.auth, UserId(u), TenantId::new(), DeviceId(d)).await),
        ErrorCode::Unauthenticated
    );
    // Revoked device.
    sqlx::query("UPDATE devices SET revoked_at = now() WHERE id = $1")
        .bind(d)
        .execute(&o.db.admin)
        .await
        .unwrap();
    assert_eq!(
        fail(build_ctx(&o.db.auth, UserId(u), t, DeviceId(d)).await),
        ErrorCode::Unauthenticated
    );
    // Removed from the tenant.
    sqlx::query("DELETE FROM memberships WHERE user_id = $1")
        .bind(other_user)
        .execute(&o.db.admin)
        .await
        .unwrap();
    assert_eq!(
        fail(build_ctx(&o.db.auth, UserId(other_user), t, DeviceId(other_dev)).await),
        ErrorCode::Unauthenticated
    );
    o.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn losing_a_scope_takes_effect_on_the_next_context() {
    let o = org().await;
    assert!(o.can_read("designer_in_finance").await.contains("finance"));
    sqlx::query("DELETE FROM scope_members WHERE user_id = $1 AND scope_id = $2")
        .bind(o.user["designer_in_finance"].0)
        .bind(o.scope["finance"])
        .execute(&o.db.admin)
        .await
        .unwrap();
    assert!(!o.can_read("designer_in_finance").await.contains("finance"));
    o.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn the_database_refuses_sealed_scopes_in_a_fold_and_sealed_folds() {
    let o = org().await;
    let q = |sql: &'static str, binds: Vec<Uuid>| {
        let db = &o.db;
        async move {
            let mut q = sqlx::query(sql);
            for b in binds {
                q = q.bind(b);
            }
            q.execute(&db.admin).await.is_err()
        }
    };
    assert!(
        q(
            "UPDATE scopes SET fold_id = $1 WHERE id = $2",
            vec![o.scope["fold"], o.scope["finance"]]
        )
        .await,
        "a sealed department must not join a fold"
    );
    assert!(
        q(
            "UPDATE scopes SET sealed = true WHERE id = $1",
            vec![o.scope["fold"]]
        )
        .await,
        "a fold is never sealed"
    );
    assert!(
        q(
            "UPDATE scopes SET fold_id = $1 WHERE id = $2",
            vec![o.scope["fold"], o.scope["fold"]]
        )
        .await,
        "a fold cannot sit inside a fold"
    );
    o.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn a_request_transaction_sees_exactly_the_rows_of_its_context() {
    let o = org().await;
    for (name, id) in &o.scope {
        if *name == "fold" {
            continue;
        }
        sqlx::query(
            "INSERT INTO entities (tenant_id, scope_id, kind, name) VALUES ($1, $2, 'person', $3)",
        )
        .bind(o.tenant)
        .bind(id)
        .bind(*name)
        .execute(&o.db.admin)
        .await
        .unwrap();
    }
    let seen = |who: &'static str| {
        let o = &o;
        async move {
            let (u, d) = o.user[who];
            let ctx = build_ctx(&o.db.auth, UserId(u), TenantId(o.tenant), DeviceId(d))
                .await
                .unwrap();
            let mut tx = begin(&o.db.app, &ctx).await.unwrap();
            let names: Vec<String> = sqlx::query_scalar("SELECT name FROM entities ORDER BY name")
                .fetch_all(&mut *tx)
                .await
                .unwrap();
            tx.rollback().await.unwrap();
            names
        }
    };
    assert_eq!(
        seen("dev").await,
        vec!["design", "marketing", "programming"]
    );
    assert_eq!(seen("finance_clerk").await, vec!["finance"]);
    assert_eq!(
        seen("ceo").await,
        vec!["design", "legal", "marketing", "operations", "programming"]
    );
    o.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn request_variables_never_leak_to_the_next_user_of_a_connection() {
    let o = org().await;
    let (u, d) = o.user["dev"];
    let ctx = build_ctx(&o.db.auth, UserId(u), TenantId(o.tenant), DeviceId(d))
        .await
        .unwrap();
    for _ in 0..6 {
        let tx = begin(&o.db.app, &ctx).await.unwrap();
        tx.commit().await.unwrap();
        // Whatever connection the pool hands out next carries nothing from the finished request.
        let left: (Option<String>, Option<String>, Option<String>) = sqlx::query_as("SELECT nullif(current_setting('app.tenant_id', true), ''), nullif(current_setting('app.user_id', true), ''), nullif(current_setting('app.scope_ids', true), '')")
            .fetch_one(&o.db.app)
            .await
            .unwrap();
        assert_eq!(left, (None, None, None));
    }
    o.db.drop_db().await.unwrap();
}
