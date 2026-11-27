#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not
//! The node's real transport against a real gateway and database: a real WebSocket and real HTTP.
//! Lives in the server crate, not the node crate, because it needs Postgres and the node's own tests must
//! pass on a machine with no database (the Windows CI runner).

use std::{sync::Arc, time::Duration};

use library_audit::AuditLog;
use library_authz::{SessionService, TokenSigner};
use library_core::{
    AuditSink, BoxFuture, Call, DeviceId, ErrorCode, LibraryError, RequestCtx, RpcHandler,
    TenantId, UserId,
};
use library_gateway::{Gateway, GatewayConfig, router};
use library_node::{
    keychain::MemoryStore,
    link::{Link, LinkState, now_secs},
    session::StoredSession,
    transport::GatewayTransport,
};
use library_testkit::{TestDb, seed_two_tenants};
use serde_json::{Value, json};
use time::OffsetDateTime;
use tokio::net::TcpListener;
use uuid::Uuid;

struct Echo;
impl RpcHandler for Echo {
    fn call<'a>(
        &'a self,
        _c: &'a RequestCtx,
        call: Call,
    ) -> BoxFuture<'a, Result<Value, LibraryError>> {
        Box::pin(async move { Ok(json!({ "method": call.method })) })
    }
}

struct World {
    gw: Arc<Gateway>,
    db: TestDb,
    base: String,
    sessions: Arc<SessionService>,
    tenant: TenantId,
    user: UserId,
    device: DeviceId,
}

async fn world() -> World {
    let db = TestDb::new().await.unwrap();
    let fx = seed_two_tenants(&db).await.unwrap();
    let sessions = Arc::new(SessionService::new(
        db.auth.clone(),
        TokenSigner::generate().unwrap(),
    ));
    // Real time: the node's own clock decides when its token is stale, so the gateway must agree with it.
    let clock = Arc::new(library_gateway::SystemClock);
    let audit: Arc<dyn AuditSink> = Arc::new(AuditLog::new(db.app.clone()));
    let gw = Arc::new(Gateway::new(
        GatewayConfig::default(),
        db.auth.clone(),
        db.app.clone(),
        sessions.clone(),
        Arc::new(Echo),
        audit,
        clock,
    ));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(gw.clone()).into_make_service_with_connect_info::<std::net::SocketAddr>();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let device: Uuid = sqlx::query_scalar("SELECT id FROM devices WHERE tenant_id = $1")
        .bind(fx.a.id)
        .fetch_one(&db.admin)
        .await
        .unwrap();
    World {
        gw,
        db,
        base: format!("http://{addr}"),
        sessions,
        tenant: TenantId(fx.a.id),
        user: UserId(fx.a.user),
        device: DeviceId(device),
    }
}

impl World {
    async fn stored(&self, expires_in: i64) -> StoredSession {
        let pair = self
            .sessions
            .start(
                self.user,
                self.tenant,
                self.device,
                OffsetDateTime::now_utc(),
            )
            .await
            .unwrap();
        StoredSession {
            tenant_id: self.tenant,
            user_id: self.user,
            access: pair.access,
            refresh: pair.refresh,
            access_expires_at: now_secs() + expires_in,
        }
    }
}

fn link(w: &World, store: Arc<MemoryStore>) -> Link<GatewayTransport> {
    Link::new(GatewayTransport::new(w.base.clone()), store)
        .unwrap()
        .0
}

#[tokio::test]
async fn a_node_signs_in_connects_and_calls_the_gateway() {
    let w = world().await;
    let store = Arc::new(MemoryStore::new());
    let l = link(&w, store.clone());
    l.sign_in(w.stored(900).await).await.unwrap();
    assert_eq!(l.state().await, LinkState::Online);
    let who = l
        .call("session.whoami", json!({}), None, Duration::from_secs(5))
        .await
        .unwrap();
    assert_eq!(who["user_id"], w.user.0.to_string());
    let catalog = l
        .call("plugins.catalog", json!({}), None, Duration::from_secs(5))
        .await
        .unwrap();
    assert_eq!(catalog["method"], "plugins.catalog");
    // A refusal arrives typed, over the real wire.
    let denied = l
        .call(
            "org.departments.upsert",
            json!({}),
            Some("key-0123456789".into()),
            Duration::from_secs(5),
        )
        .await
        .unwrap_err();
    assert_eq!(
        (denied.code, denied.retryable),
        (ErrorCode::PermissionDenied, false)
    );
    // And the write that was refused for role still did not need a key to be refused for it.
    let nokey = l
        .call("usage.report", json!({}), None, Duration::from_secs(5))
        .await
        .unwrap_err();
    assert_eq!(nokey.code, ErrorCode::InvalidParams);
    w.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn a_node_that_was_asleep_for_hours_refreshes_over_http_and_reconnects() {
    let w = world().await;
    let store = Arc::new(MemoryStore::new());
    // After a weekend with the lid closed: the access token is long dead, the refresh token is still good.
    let mut asleep = w.stored(900).await;
    let dead_access =
        "v1.thisIsNoLongerAValidAccessTokenAtAll.signatureSignatureSignature".to_string();
    asleep.access = dead_access.clone();
    asleep.access_expires_at = now_secs() - 3 * 24 * 3600;
    asleep.save(&*store).unwrap();
    let l = link(&w, store.clone());
    l.connect(now_secs()).await.unwrap();
    assert_eq!(l.state().await, LinkState::Online);
    assert_eq!(
        l.call("session.whoami", json!({}), None, Duration::from_secs(5))
            .await
            .unwrap()["tenant_id"],
        w.tenant.0.to_string()
    );
    let after = StoredSession::load(&*store).unwrap().unwrap();
    assert_ne!(
        after.access, dead_access,
        "the new pair replaced the dead one in the keychain"
    );
    assert_ne!(
        after.refresh, asleep.refresh,
        "and the spent refresh token was rotated"
    );
    w.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn a_revoked_device_is_signed_out_by_its_next_refresh_and_the_keychain_is_wiped() {
    let w = world().await;
    let store = Arc::new(MemoryStore::new());
    let mut s = w.stored(900).await;
    s.access_expires_at = now_secs() - 10;
    s.save(&*store).unwrap();
    sqlx::query("UPDATE devices SET revoked_at = now() WHERE id = $1")
        .bind(w.device.0)
        .execute(&w.db.admin)
        .await
        .unwrap();
    let l = link(&w, store.clone());
    let e = l.connect(now_secs()).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::Unauthenticated);
    assert_eq!(l.state().await, LinkState::SignedOut);
    assert!(StoredSession::load(&*store).unwrap().is_none());
    w.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn a_gateway_that_is_down_leaves_the_node_offline_with_its_session_intact() {
    let w = world().await;
    let store = Arc::new(MemoryStore::new());
    let mut s = w.stored(900).await;
    s.access_expires_at = now_secs() - 10;
    s.save(&*store).unwrap();
    // Nothing listens on this port.
    let dead = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", dead.local_addr().unwrap());
    drop(dead);
    let l = Link::new(GatewayTransport::new(base), store.clone())
        .unwrap()
        .0;
    let started = std::time::Instant::now();
    assert_eq!(
        l.connect(now_secs()).await.unwrap_err().code,
        ErrorCode::Unavailable
    );
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "an unreachable gateway is reported promptly"
    );
    assert_eq!(l.state().await, LinkState::Offline);
    assert!(
        StoredSession::load(&*store).unwrap().is_some(),
        "offline is not signed out"
    );
    w.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn pushes_from_the_gateway_arrive_on_the_nodes_notification_channel() {
    let w = world().await;
    let (l, mut pushes) = Link::new(
        GatewayTransport::new(w.base.clone()),
        Arc::new(MemoryStore::new()),
    )
    .unwrap();
    l.sign_in(w.stored(900).await).await.unwrap();
    // A call has been served, so the connection is registered with the gateway's hub.
    l.call("session.whoami", json!({}), None, Duration::from_secs(5))
        .await
        .unwrap();
    assert!(
        w.gw.hub()
            .notify(w.device, "scope.revoked", json!({ "scope_id": "abc" }))
    );
    let (name, params) = tokio::time::timeout(Duration::from_secs(5), pushes.recv())
        .await
        .expect("the push must arrive")
        .unwrap();
    assert_eq!(
        (name.as_str(), params["scope_id"].as_str()),
        ("scope.revoked", Some("abc"))
    );
    // Hanging up from the gateway side takes the node offline, and its session stays.
    assert!(w.gw.hub().disconnect(w.device));
    for _ in 0..100 {
        if l.state().await == LinkState::Offline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(l.state().await, LinkState::Offline);
    assert_eq!(
        l.call("session.whoami", json!({}), None, Duration::from_secs(2))
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unavailable
    );
    w.db.drop_db().await.unwrap();
}
