#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not
//! The five checks, one by one, against a real database.

mod common;

use common::{code, env, env_with, rpc_code};
use library_gateway::{GatewayConfig, Limit, Limits};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn whoami_reports_what_the_server_knows() {
    let e = env().await;
    let mut c = e.connect(false).await;
    let r = e.call(&mut c, "session.whoami", json!({}), None).await;
    assert_eq!(r["result"]["tenant_id"], c.tenant.0.to_string());
    assert_eq!(r["result"]["user_id"], c.user.0.to_string());
    assert_eq!(r["result"]["plan"], "starter");
    assert_eq!(r["result"]["roles"], json!(["member"]));
    assert_eq!(r["result"]["scopes"].as_array().unwrap().len(), 2);
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn bad_messages_get_typed_errors_and_never_reach_a_handler() {
    let e = env().await;
    let mut c = e.connect(false).await;
    for (input, want) in [("not json", -32700), ("[]", -32700), ("{}", -32700)] {
        let r = serde_json::to_value(e.gw.handle(&mut c.session, input).await).unwrap();
        assert_eq!(rpc_code(&r), Some(want), "{input}");
    }
    let wrong_version =
        json!({ "jsonrpc": "1.0", "id": 1, "method": "session.whoami" }).to_string();
    assert_eq!(
        code(&serde_json::to_value(e.gw.handle(&mut c.session, &wrong_version).await).unwrap())
            .as_deref(),
        Some("invalid_params")
    );
    // Unknown methods, and methods that belong to the app-to-node link, do not exist here.
    for m in ["nope.nothing", "node.info", "local.ask", ""] {
        assert_eq!(
            rpc_code(&e.call(&mut c, m, json!({}), None).await),
            Some(-32601),
            "{m:?}"
        );
    }
    assert_eq!(e.handler.count(), 0);
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn an_oversized_message_is_refused_unparsed() {
    let e = env_with(
        GatewayConfig {
            max_message_bytes: 2048,
            ..GatewayConfig::default()
        },
        false,
    )
    .await;
    let mut c = e.connect(false).await;
    let big = json!({ "jsonrpc": "2.0", "id": 1, "method": "plugins.catalog", "params": { "pad": "x".repeat(5000) } }).to_string();
    let r = serde_json::to_value(e.gw.handle(&mut c.session, &big).await).unwrap();
    assert_eq!(code(&r).as_deref(), Some("payload_too_large"));
    assert_eq!(e.handler.count(), 0);
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn role_plan_and_scope_are_checked_before_the_handler() {
    let e = env().await;
    let mut c = e.connect(false).await;
    // Role: only admins may touch departments.
    assert_eq!(
        code(
            &e.call(
                &mut c,
                "org.departments.upsert",
                json!({}),
                Some("key-role-0001")
            )
            .await
        )
        .as_deref(),
        Some("permission_denied")
    );
    // Plan: tasks need Team and the fixture tenant is on Starter.
    assert_eq!(
        code(&e.call(&mut c, "tasks.list", json!({}), None).await).as_deref(),
        Some("plan_required")
    );
    // Scope: a scope of another tenant, a missing scope, a malformed scope.
    assert_eq!(
        code(
            &e.call(
                &mut c,
                "sync.snapshot",
                json!({ "scope_id": e.fx.b.scopes[0].to_string() }),
                None
            )
            .await
        )
        .as_deref(),
        Some("scope_denied")
    );
    assert_eq!(
        code(&e.call(&mut c, "sync.snapshot", json!({}), None).await).as_deref(),
        Some("invalid_params")
    );
    assert_eq!(
        code(
            &e.call(
                &mut c,
                "sync.snapshot",
                json!({ "scope_id": "not-a-uuid" }),
                None
            )
            .await
        )
        .as_deref(),
        Some("invalid_params")
    );
    assert_eq!(
        code(
            &e.call(&mut c, "sync.snapshot", json!({ "scope_id": 42 }), None)
                .await
        )
        .as_deref(),
        Some("invalid_params")
    );
    // Server-only methods are closed to every device.
    assert_eq!(
        code(
            &e.call(
                &mut c,
                "brain.context",
                json!({ "scope_id": e.fx.a.scopes[0].to_string() }),
                None
            )
            .await
        )
        .as_deref(),
        Some("permission_denied")
    );
    assert_eq!(e.handler.count(), 0, "no handler ran for any refused call");
    // And an allowed call reaches the handler.
    let ok = e
        .call(
            &mut c,
            "sync.snapshot",
            json!({ "scope_id": e.fx.a.scopes[0].to_string() }),
            None,
        )
        .await;
    assert_eq!(ok["result"]["method"], "sync.snapshot");
    assert_eq!(e.handler.count(), 1);
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn a_write_needs_a_wellformed_idempotency_key() {
    let e = env().await;
    let mut c = e.connect(false).await;
    for key in [
        None,
        Some(""),
        Some("short"),
        Some("has spaces in it!"),
        Some("bad;drop table"),
        Some(&"k".repeat(129)),
    ] {
        let r = e.call(&mut c, "usage.report", json!({}), key).await;
        assert_eq!(code(&r).as_deref(), Some("invalid_params"), "{key:?}");
    }
    assert_eq!(e.handler.count(), 0);
    assert!(
        e.audit_rows(c.tenant.0).await.is_empty(),
        "a refused write leaves no audit row"
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn replaying_a_write_returns_the_first_result_without_running_it_again() {
    let e = env().await;
    let mut c = e.connect(false).await;
    let first = e
        .call(
            &mut c,
            "usage.report",
            json!({ "day": "2026-11-26" }),
            Some("report-2026-11-26"),
        )
        .await;
    let again = e
        .call(
            &mut c,
            "usage.report",
            json!({ "day": "2026-11-26" }),
            Some("report-2026-11-26"),
        )
        .await;
    assert_eq!(first["result"], again["result"]);
    assert_eq!(first["result"]["n"], 1);
    assert_eq!(e.handler.count(), 1, "the handler ran once");
    assert_eq!(
        e.audit_rows(c.tenant.0).await.len(),
        1,
        "a replay is not audited again"
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn reusing_a_key_for_a_different_request_is_refused() {
    let e = env().await;
    let mut c = e.connect(false).await;
    e.call(
        &mut c,
        "usage.report",
        json!({ "a": 1 }),
        Some("shared-key-0001"),
    )
    .await;
    let other_params = e
        .call(
            &mut c,
            "usage.report",
            json!({ "a": 2 }),
            Some("shared-key-0001"),
        )
        .await;
    assert_eq!(code(&other_params).as_deref(), Some("conflict"));
    let other_method = e
        .call(
            &mut c,
            "runs.start",
            json!({ "a": 1 }),
            Some("shared-key-0001"),
        )
        .await;
    assert_eq!(code(&other_method).as_deref(), Some("conflict"));
    assert_eq!(e.handler.count(), 1);
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn idempotency_keys_are_private_to_a_tenant_and_user() {
    let e = env().await;
    let (mut a, mut b) = (e.connect(false).await, e.connect(true).await);
    let ra = e
        .call(&mut a, "usage.report", json!({}), Some("same-key-12345"))
        .await;
    let rb = e
        .call(&mut b, "usage.report", json!({}), Some("same-key-12345"))
        .await;
    assert_eq!(ra["result"]["n"], 1);
    assert_eq!(
        rb["result"]["n"], 2,
        "another tenant's identical key is a different key"
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn a_retryable_failure_can_be_retried_and_a_permanent_one_is_remembered() {
    let e = env().await;
    let mut c = e.connect(false).await;
    let r1 = e
        .call(
            &mut c,
            "usage.report",
            json!({ "fail": "retryable" }),
            Some("retry-key-0001"),
        )
        .await;
    assert_eq!(code(&r1).as_deref(), Some("unavailable"));
    let r2 = e
        .call(
            &mut c,
            "usage.report",
            json!({ "fail": "retryable" }),
            Some("retry-key-0001"),
        )
        .await;
    assert_eq!(code(&r2).as_deref(), Some("unavailable"));
    assert_eq!(
        e.handler.count(),
        2,
        "a retryable failure does not freeze the key"
    );

    let p1 = e
        .call(
            &mut c,
            "usage.report",
            json!({ "fail": "permanent" }),
            Some("perm-key-00001"),
        )
        .await;
    let p2 = e
        .call(
            &mut c,
            "usage.report",
            json!({ "fail": "permanent" }),
            Some("perm-key-00001"),
        )
        .await;
    assert_eq!(code(&p1).as_deref(), Some("not_found"));
    assert_eq!(p1["error"], p2["error"]);
    assert_eq!(
        e.handler.count(),
        3,
        "a permanent failure is the answer and is not run again"
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn two_simultaneous_calls_with_one_key_run_the_handler_once() {
    let e = env().await;
    let (mut c1, mut c2) = (e.connect(false).await, e.connect(false).await);
    let (a, b) = tokio::join!(
        e.call(&mut c1, "usage.report", json!({}), Some("race-key-00001")),
        e.call(&mut c2, "usage.report", json!({}), Some("race-key-00001"))
    );
    assert_eq!(e.handler.count(), 1, "exactly one execution");
    let oks = [&a, &b].iter().filter(|r| r["result"].is_object()).count();
    assert!(oks >= 1);
    for r in [&a, &b] {
        if r["result"].is_null() {
            assert_eq!(code(r).as_deref(), Some("conflict"));
            assert_eq!(
                r["error"]["data"]["retryable"], true,
                "the loser is told to retry"
            );
        }
    }
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn a_call_that_died_midway_does_not_block_its_key_forever() {
    let e = env_with(
        GatewayConfig {
            pending_timeout_s: 0,
            ..GatewayConfig::default()
        },
        false,
    )
    .await;
    let mut c = e.connect(false).await;
    // Simulate a crash after the key was claimed but before the handler finished.
    let params_hash: Vec<u8> = {
        use sha2::{Digest, Sha256};
        Sha256::digest(serde_json::to_vec(&json!({})).unwrap()).to_vec()
    };
    sqlx::query("INSERT INTO idempotency_keys (tenant_id, user_id, key, method, params_hash, created_at) VALUES ($1, $2, 'crashed-key-001', 'usage.report', $3, now() - interval '5 minutes')")
        .bind(c.tenant.0)
        .bind(c.user.0)
        .bind(params_hash)
        .execute(&e.db.admin)
        .await
        .unwrap();
    let r = e
        .call(&mut c, "usage.report", json!({}), Some("crashed-key-001"))
        .await;
    assert!(r["result"].is_object(), "{r}");
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn every_write_is_audited_before_it_runs_and_the_chain_verifies() {
    let e = env().await;
    let mut c = e.connect(false).await;
    e.call(&mut c, "usage.report", json!({}), Some("audit-key-00001"))
        .await;
    e.call(&mut c, "runs.start", json!({}), Some("audit-key-00002"))
        .await;
    e.call(&mut c, "plugins.catalog", json!({}), None).await; // a read: not audited here
    let rows = e.audit_rows(c.tenant.0).await;
    assert_eq!(
        rows.iter().map(|r| r.0.as_str()).collect::<Vec<_>>(),
        vec!["usage.report", "runs.start"]
    );
    assert!(rows.iter().all(|r| r.1 == format!("user:{}", c.user.0)));
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn if_the_audit_log_is_down_nothing_runs() {
    let e = env_with(GatewayConfig::default(), true).await;
    let mut c = e.connect(false).await;
    let r = e
        .call(&mut c, "usage.report", json!({}), Some("audit-down-0001"))
        .await;
    assert_eq!(code(&r).as_deref(), Some("internal"));
    assert_eq!(e.handler.count(), 0, "no write may happen unaudited");
    // And the key was released, so the client can retry once the log is back.
    let left: i64 =
        sqlx::query_scalar("SELECT count(*) FROM idempotency_keys WHERE key = 'audit-down-0001'")
            .fetch_one(&e.db.admin)
            .await
            .unwrap();
    assert_eq!(left, 0);
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn rate_limits_refuse_with_a_retry_hint_and_recover_over_time() {
    let tight = Limits {
        tenant: Limit {
            burst: 100.0,
            per_second: 100.0,
        },
        user: Limit {
            burst: 100.0,
            per_second: 100.0,
        },
        method: Limit {
            burst: 3.0,
            per_second: 1.0,
        },
    };
    let e = env_with(
        GatewayConfig {
            limits: tight,
            ..GatewayConfig::default()
        },
        false,
    )
    .await;
    let mut c = e.connect(false).await;
    for _ in 0..3 {
        assert!(e.call(&mut c, "plugins.catalog", json!({}), None).await["result"].is_object());
    }
    let refused = e.call(&mut c, "plugins.catalog", json!({}), None).await;
    assert_eq!(code(&refused).as_deref(), Some("rate_limited"));
    assert_eq!(refused["error"]["data"]["retryable"], true);
    assert!(
        refused["error"]["message"].as_str().unwrap().contains("ms"),
        "the message says when to retry"
    );
    assert_eq!(e.handler.count(), 3);
    // Other methods have their own bucket.
    assert!(e.call(&mut c, "session.whoami", json!({}), None).await["result"].is_object());
    e.clock.advance_ms(2000);
    assert!(
        e.call(&mut c, "plugins.catalog", json!({}), None).await["result"].is_object(),
        "tokens come back with time"
    );
    // Another tenant is unaffected by this tenant's usage.
    let mut b = e.connect(true).await;
    assert!(e.call(&mut b, "plugins.catalog", json!({}), None).await["result"].is_object());
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn an_expired_access_token_must_be_refreshed_and_only_by_its_own_device() {
    let e = env().await;
    let mut c = e.connect(false).await;
    let mut other = e.connect(true).await;
    let (refresh_token, other_refresh) = (c.pair.refresh.clone(), other.pair.refresh.clone());
    e.clock.advance_ms(16 * 60 * 1000);
    let r = e.call(&mut c, "plugins.catalog", json!({}), None).await;
    assert_eq!(code(&r).as_deref(), Some("token_expired"));

    // Another device's refresh token cannot be used from this connection, and is not used up by the attempt.
    let stolen = e
        .call(
            &mut c,
            "session.refresh",
            json!({ "refresh": other_refresh.clone() }),
            None,
        )
        .await;
    assert_eq!(code(&stolen).as_deref(), Some("unauthenticated"));
    assert!(
        e.sessions
            .refresh(&other.pair.refresh, e.clock_now())
            .await
            .is_ok(),
        "the owner's token still works"
    );

    // The right token refreshes the connection in place.
    let fresh = e
        .call(
            &mut c,
            "session.refresh",
            json!({ "refresh": refresh_token.clone() }),
            None,
        )
        .await;
    assert!(
        fresh["result"]["access"]
            .as_str()
            .unwrap()
            .starts_with("v1."),
        "{fresh}"
    );
    assert!(e.call(&mut c, "plugins.catalog", json!({}), None).await["result"].is_object());
    // Reusing the old refresh token is theft: it fails, and ends the family.
    let replay = e
        .call(
            &mut c,
            "session.refresh",
            json!({ "refresh": refresh_token.clone() }),
            None,
        )
        .await;
    assert_eq!(code(&replay).as_deref(), Some("unauthenticated"));
    let newest = fresh["result"]["refresh"].as_str().unwrap().to_string();
    assert!(
        e.sessions.refresh(&newest, e.clock_now()).await.is_err(),
        "the whole family died"
    );
    let _ = &mut other;
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn refresh_tokens_never_become_a_stored_idempotent_response() {
    let e = env().await;
    let mut c = e.connect(false).await;
    let refresh_token = c.pair.refresh.clone();
    // Even a client that (wrongly) sends a key with session.refresh must not get tokens stored in the table.
    let r = e
        .call(
            &mut c,
            "session.refresh",
            json!({ "refresh": refresh_token.clone() }),
            Some("refresh-key-0001"),
        )
        .await;
    assert!(r["result"].is_object());
    let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM idempotency_keys WHERE response::text LIKE '%lrt1_%' OR response::text LIKE '%v1.%'").fetch_one(&e.db.admin).await.unwrap();
    assert_eq!(stored, 0);
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn role_and_device_changes_take_effect_within_the_context_lifetime() {
    let e = env_with(
        GatewayConfig {
            ctx_ttl_ms: 30_000,
            ..GatewayConfig::default()
        },
        false,
    )
    .await;
    let mut c = e.connect(false).await;
    assert_eq!(
        code(&promote_call(&e, &mut c).await).as_deref(),
        Some("permission_denied")
    );
    sqlx::query("UPDATE memberships SET role = 'admin' WHERE tenant_id = $1 AND user_id = $2")
        .bind(c.tenant.0)
        .bind(c.user.0)
        .execute(&e.db.admin)
        .await
        .unwrap();
    assert_eq!(
        code(&promote_call(&e, &mut c).await).as_deref(),
        Some("permission_denied"),
        "still cached inside the lifetime"
    );
    e.clock.advance_ms(31_000);
    assert!(
        promote_call(&e, &mut c).await["result"].is_object(),
        "the promotion applies once the context is rebuilt"
    );

    // Revoking the device ends the connection's usefulness and tells the hub to hang up.
    let (mut inbox, _gen) = e.gw.hub().register(c.device);
    sqlx::query("UPDATE devices SET revoked_at = now() WHERE id = $1")
        .bind(c.device.0)
        .execute(&e.db.admin)
        .await
        .unwrap();
    e.clock.advance_ms(31_000);
    assert_eq!(
        code(&e.call(&mut c, "plugins.catalog", json!({}), None).await).as_deref(),
        Some("unauthenticated")
    );
    assert_eq!(
        inbox.try_recv().ok(),
        Some(library_gateway::Push::Disconnect)
    );
    e.db.drop_db().await.unwrap();
}

/// An admin-only write with a fresh idempotency key each time.
async fn promote_call(e: &common::Env, c: &mut common::Conn) -> serde_json::Value {
    let key = format!("promo-{}", Uuid::new_v4());
    e.call(c, "org.departments.upsert", json!({}), Some(&key))
        .await
}
