//! Test support, not a product binary: starts a real gateway on a local port over a throwaway database and
//! prints one JSON line with everything a test needs to sign a node in. Stays up until stdin closes.
//!
//! Used by the desktop end-to-end test. Needs the dev Postgres (`just dev-up`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{io::Read, sync::Arc};

use library_audit::AuditLog;
use library_authz::{SessionService, TokenSigner};
use library_core::{
    AuditSink, BoxFuture, Call, DeviceId, LibraryError, RequestCtx, RpcHandler, TenantId, UserId,
};
use library_gateway::{Gateway, GatewayConfig, SystemClock, router};
use library_testkit::{TestDb, seed_two_tenants};
use serde_json::{Value, json};
use time::OffsetDateTime;

struct Echo;

impl RpcHandler for Echo {
    fn call<'a>(
        &'a self,
        _c: &'a RequestCtx,
        call: Call,
    ) -> BoxFuture<'a, Result<Value, LibraryError>> {
        Box::pin(async move { Ok(json!({ "echo": call.method })) })
    }
}

#[tokio::main]
async fn main() {
    let db = TestDb::new().await.unwrap();
    let fx = seed_two_tenants(&db).await.unwrap();
    let sessions = Arc::new(SessionService::new(
        db.auth.clone(),
        TokenSigner::generate().unwrap(),
    ));
    let audit: Arc<dyn AuditSink> = Arc::new(AuditLog::new(db.app.clone()));
    let gw = Arc::new(Gateway::new(
        GatewayConfig::default(),
        db.auth.clone(),
        db.app.clone(),
        sessions.clone(),
        Arc::new(Echo),
        audit,
        Arc::new(SystemClock),
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(gw).into_make_service_with_connect_info::<std::net::SocketAddr>();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let device: uuid::Uuid = sqlx::query_scalar("SELECT id FROM devices WHERE tenant_id = $1")
        .bind(fx.a.id)
        .fetch_one(&db.admin)
        .await
        .unwrap();
    let pair = sessions
        .start(
            UserId(fx.a.user),
            TenantId(fx.a.id),
            DeviceId(device),
            OffsetDateTime::now_utc(),
        )
        .await
        .unwrap();
    println!(
        "{}",
        json!({
            "url": format!("http://{addr}"),
            "tenant_id": fx.a.id, "user_id": fx.a.user, "device_id": device,
            "access": pair.access, "refresh": pair.refresh, "access_expires_at": pair.access_expires_at.unix_timestamp(),
            "foreign_scope": fx.b.scopes[0],
        })
    );
    // Stay up until the test closes our stdin, then clean up the database.
    tokio::task::spawn_blocking(|| {
        let mut sink = Vec::new();
        let _ = std::io::stdin().read_to_end(&mut sink);
    })
    .await
    .unwrap();
    db.drop_db().await.unwrap();
}
