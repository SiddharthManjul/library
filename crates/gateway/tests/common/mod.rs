#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
//! Shared fixture: a real database, a real session service, a gateway with a recording handler.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use library_audit::AuditLog;
use library_authz::{SessionService, TokenPair, TokenSigner};
use library_core::{
    AuditRecord, AuditSink, BoxFuture, Call, DeviceId, ErrorCode, LibraryError, RequestCtx,
    RpcHandler, TenantId, UserId,
};
use library_gateway::{Gateway, GatewayConfig, ManualClock, Session};
use library_testkit::{Fixture, TestDb, seed_two_tenants};
use serde_json::{Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

/// Records every call. `params.fail` = "retryable" or "permanent" makes it fail.
#[derive(Default)]
pub struct Recorder {
    pub calls: Mutex<Vec<(String, Value)>>,
    pub count: AtomicUsize,
}

impl Recorder {
    pub fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

impl RpcHandler for Recorder {
    fn call<'a>(
        &'a self,
        _ctx: &'a RequestCtx,
        call: Call,
    ) -> BoxFuture<'a, Result<Value, LibraryError>> {
        Box::pin(async move {
            let n = self.count.fetch_add(1, Ordering::SeqCst) + 1;
            self.calls
                .lock()
                .unwrap()
                .push((call.method.clone(), call.params.clone()));
            match call.params.get("fail").and_then(Value::as_str) {
                Some("retryable") => Err(LibraryError::new(ErrorCode::Unavailable, "try again")),
                Some("permanent") => Err(LibraryError::new(ErrorCode::NotFound, "no such thing")),
                _ => Ok(json!({ "n": n, "method": call.method })),
            }
        })
    }
}

/// An audit sink that always fails, to prove nothing runs unaudited.
pub struct BrokenAudit;

impl AuditSink for BrokenAudit {
    fn record<'a>(
        &'a self,
        _ctx: &'a RequestCtx,
        _r: AuditRecord,
    ) -> BoxFuture<'a, Result<(), LibraryError>> {
        Box::pin(async { Err(LibraryError::internal()) })
    }
}

pub struct Env {
    pub db: TestDb,
    pub fx: Fixture,
    pub gw: Arc<Gateway>,
    pub handler: Arc<Recorder>,
    pub sessions: Arc<SessionService>,
    pub clock: Arc<ManualClock>,
    pub audit: Arc<AuditLog>,
}

pub struct Conn {
    pub session: Session,
    pub pair: TokenPair,
    pub device: DeviceId,
    pub tenant: TenantId,
    pub user: UserId,
}

pub async fn env_with(cfg: GatewayConfig, broken_audit: bool) -> Env {
    let db = TestDb::new().await.unwrap();
    let fx = seed_two_tenants(&db).await.unwrap();
    // Start from empty audit logs so counts are ours.
    sqlx::query("ALTER TABLE audit_log DISABLE TRIGGER USER")
        .execute(&db.admin)
        .await
        .unwrap();
    sqlx::query("DELETE FROM audit_log")
        .execute(&db.admin)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE audit_log ENABLE TRIGGER USER")
        .execute(&db.admin)
        .await
        .unwrap();
    let sessions = Arc::new(SessionService::new(
        db.auth.clone(),
        TokenSigner::generate().unwrap(),
    ));
    let handler = Arc::new(Recorder::default());
    let clock = Arc::new(ManualClock::new(OffsetDateTime::now_utc()));
    let audit = Arc::new(AuditLog::new(db.app.clone()));
    let sink: Arc<dyn AuditSink> = if broken_audit {
        Arc::new(BrokenAudit)
    } else {
        audit.clone()
    };
    let gw = Arc::new(Gateway::new(
        cfg,
        db.auth.clone(),
        db.app.clone(),
        sessions.clone(),
        handler.clone(),
        sink,
        clock.clone(),
    ));
    Env {
        db,
        fx,
        gw,
        handler,
        sessions,
        clock,
        audit,
    }
}

pub async fn env() -> Env {
    env_with(GatewayConfig::default(), false).await
}

impl Env {
    /// Signs in the fixture's member of tenant A (or B) and opens a gateway session for them.
    pub async fn connect(&self, tenant_b: bool) -> Conn {
        let t = if tenant_b { &self.fx.b } else { &self.fx.a };
        let device: Uuid = sqlx::query_scalar("SELECT id FROM devices WHERE tenant_id = $1")
            .bind(t.id)
            .fetch_one(&self.db.admin)
            .await
            .unwrap();
        let (tenant, user, device) = (TenantId(t.id), UserId(t.user), DeviceId(device));
        let pair = self
            .sessions
            .start(user, tenant, device, self.clock_now())
            .await
            .unwrap();
        let session = self.gw.open(&pair.access).await.unwrap();
        Conn {
            session,
            pair,
            device,
            tenant,
            user,
        }
    }

    pub fn clock_now(&self) -> OffsetDateTime {
        use library_gateway::Clock;
        self.clock.now()
    }

    pub async fn call(
        &self,
        c: &mut Conn,
        method: &str,
        params: Value,
        key: Option<&str>,
    ) -> Value {
        let mut req = json!({ "jsonrpc": "2.0", "id": 7, "method": method, "params": params });
        if let Some(k) = key {
            req["idempotency_key"] = json!(k);
        }
        let resp = self.gw.handle(&mut c.session, &req.to_string()).await;
        serde_json::to_value(resp).unwrap()
    }

    pub async fn audit_rows(&self, tenant: Uuid) -> Vec<(String, String)> {
        sqlx::query_as("SELECT action, actor FROM audit_log WHERE tenant_id = $1 ORDER BY seq")
            .bind(tenant)
            .fetch_all(&self.db.admin)
            .await
            .unwrap()
    }
}

/// The typed error code of a response, if it is an error.
pub fn code(resp: &Value) -> Option<String> {
    resp["error"]["data"]["code"].as_str().map(str::to_string)
}

pub fn rpc_code(resp: &Value) -> Option<i64> {
    resp["error"]["code"].as_i64()
}
