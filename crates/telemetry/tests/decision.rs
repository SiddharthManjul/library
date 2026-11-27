#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not
//! The decision log: a closed set of kinds, scrubbed inputs, append only, private to a tenant.

use library_core::{
    ErrorCode, Plan, Principal, RequestCtx, Role, Taint, TenantId, TraceId, UserId,
};
use library_telemetry::{DecisionKind, DecisionLog};
use library_testkit::{TestDb, seed_two_tenants};
use serde_json::json;
use uuid::Uuid;

fn ctx(tenant: Uuid, user: Uuid) -> RequestCtx {
    RequestCtx {
        tenant_id: TenantId(tenant),
        principal: Principal::User {
            user_id: UserId(user),
        },
        roles: vec![Role::Member],
        scopes: vec![],
        plan: Plan::Team,
        taint: Taint::Clean,
        trace_id: TraceId::new(),
        device_id: None,
    }
}

async fn setup() -> (TestDb, DecisionLog, RequestCtx, RequestCtx) {
    let db = TestDb::new().await.unwrap();
    let fx = seed_two_tenants(&db).await.unwrap();
    sqlx::query("DELETE FROM decision_log")
        .execute(&db.admin)
        .await
        .unwrap();
    let log = DecisionLog::new(db.app.clone());
    (db, log, ctx(fx.a.id, fx.a.user), ctx(fx.b.id, fx.b.user))
}

#[tokio::test]
async fn every_kind_can_be_recorded_with_its_inputs() {
    let (db, log, a, _b) = setup().await;
    let run = Uuid::new_v4();
    for kind in [
        DecisionKind::Routing,
        DecisionKind::Decomposition,
        DecisionKind::ApprovalTier,
        DecisionKind::Lease,
        DecisionKind::AdvisorCall,
        DecisionKind::RelayAssignment,
        DecisionKind::ChangeTrigger,
    ] {
        log.record(
            &a,
            Some(run),
            kind,
            &json!({ "b": 3, "s": 0.2, "w": 0 }),
            "fan_out",
            Some("ok"),
        )
        .await
        .unwrap();
    }
    let kinds: Vec<String> = sqlx::query_scalar(
        "SELECT kind FROM decision_log WHERE tenant_id = $1 ORDER BY created_at, kind",
    )
    .bind(a.tenant_id.0)
    .fetch_all(&db.admin)
    .await
    .unwrap();
    assert_eq!(kinds.len(), 7);
    let stored: serde_json::Value =
        sqlx::query_scalar("SELECT inputs FROM decision_log WHERE kind = 'decomposition'")
            .fetch_one(&db.admin)
            .await
            .unwrap();
    assert_eq!(stored["b"], 3);
    db.drop_db().await.unwrap();
}

#[tokio::test]
async fn the_database_refuses_a_kind_outside_the_closed_set() {
    let (db, _log, a, _b) = setup().await;
    let r = sqlx::query("INSERT INTO decision_log (tenant_id, kind, inputs, choice) VALUES ($1, 'made_up_kind', '{}', 'x')").bind(a.tenant_id.0).execute(&db.admin).await;
    assert!(r.is_err());
    db.drop_db().await.unwrap();
}

#[tokio::test]
async fn credential_shaped_inputs_are_scrubbed_before_they_are_stored() {
    let (db, log, a, _b) = setup().await;
    let inputs = json!({ "model": "claude", "api_key": "sk-ant-api03-abcdefghijklmnopqrstuvwxyz", "note": "retry with lrt1_Zm9vYmFyYmF6cXV4Zm9vYmFyYmF6cXV4Zm9v", "nested": { "token": "abc" } });
    log.record(
        &a,
        None,
        DecisionKind::Routing,
        &inputs,
        "use lrt1_Zm9vYmFyYmF6cXV4Zm9vYmFyYmF6cXV4Zm9v",
        Some("password=hunter2"),
    )
    .await
    .unwrap();
    let row: (String, Option<String>, String) =
        sqlx::query_as("SELECT inputs::text, outcome, choice FROM decision_log")
            .fetch_one(&db.admin)
            .await
            .unwrap();
    let all = format!("{} {:?} {}", row.0, row.1, row.2);
    for leaked in ["sk-ant-api03", "lrt1_", "hunter2", "\"abc\""] {
        assert!(!all.contains(leaked), "{leaked} leaked: {all}");
    }
    assert!(all.contains("claude"), "ordinary inputs survive: {all}");
    db.drop_db().await.unwrap();
}

#[tokio::test]
async fn oversized_inputs_are_refused() {
    let (db, log, a, _b) = setup().await;
    let big = json!({ "dump": "x".repeat(20_000) });
    assert_eq!(
        log.record(&a, None, DecisionKind::Routing, &big, "x", None)
            .await
            .unwrap_err()
            .code,
        ErrorCode::PayloadTooLarge
    );
    db.drop_db().await.unwrap();
}

#[tokio::test]
async fn tenants_cannot_see_or_change_each_others_decisions_and_nothing_can_be_edited() {
    let (db, log, a, b) = setup().await;
    log.record(&a, None, DecisionKind::Routing, &json!({}), "a", None)
        .await
        .unwrap();
    log.record(&b, None, DecisionKind::Routing, &json!({}), "b", None)
        .await
        .unwrap();
    let mut tx = library_authz::begin(&db.app, &a).await.unwrap();
    let seen: Vec<String> = sqlx::query_scalar("SELECT choice FROM decision_log")
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    assert_eq!(seen, vec!["a"]);
    assert!(
        sqlx::query("UPDATE decision_log SET choice = 'x'")
            .execute(&mut *tx)
            .await
            .is_err(),
        "the log is append only"
    );
    // The failed statement left the transaction aborted. End it and wait, so its connection is back in the
    // pool before the pool is closed: closing a pool waits for every connection to be returned.
    tx.rollback().await.unwrap();
    db.drop_db().await.unwrap();
    let _ = b;
}
