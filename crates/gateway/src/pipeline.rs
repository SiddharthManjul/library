use std::sync::Arc;

use library_authz::{SessionService, TokenVerifier, build_ctx};
use library_core::{
    AuditRecord, AuditSink, Call, ErrorCode, LibraryError, RequestCtx, RpcHandler, ScopeId,
};
use library_protocol::{
    Access, Link, RpcRequest, RpcResponse, TokenPairWire, WhoAmI, authorize, find,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    Clock, Hub, Limits, RateLimiter,
    idempotency::{self, Claim},
};

#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// Larger messages are refused before they are parsed.
    pub max_message_bytes: usize,
    /// How long a connection may reuse its context before roles, scopes and the device are checked again.
    pub ctx_ttl_ms: i64,
    /// A write that has been "in progress" longer than this is assumed dead and may be retried.
    pub pending_timeout_s: i64,
    pub limits: Limits,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            max_message_bytes: 1024 * 1024,
            ctx_ttl_ms: 30_000,
            pending_timeout_s: 60,
            limits: Limits::default(),
        }
    }
}

/// State of one authenticated connection.
pub struct Session {
    pub ctx: RequestCtx,
    access_exp: i64,
    ctx_built_at: OffsetDateTime,
}

pub struct Gateway {
    pub(crate) cfg: GatewayConfig,
    auth: PgPool,
    app: PgPool,
    sessions: Arc<SessionService>,
    verifier: TokenVerifier,
    handler: Arc<dyn RpcHandler>,
    audit: Arc<dyn AuditSink>,
    limiter: RateLimiter,
    pub(crate) hub: Hub,
    clock: Arc<dyn Clock>,
}

fn unauthenticated() -> LibraryError {
    LibraryError::new(ErrorCode::Unauthenticated, "Sign in again to continue.")
}

fn payload_hash(params: &Value) -> [u8; 32] {
    Sha256::digest(serde_json::to_vec(params).unwrap_or_default()).into()
}

/// How a request failed: a typed library error, or "no such method" (a JSON-RPC level error).
enum Fail {
    NoMethod,
    Error(LibraryError),
}

impl From<LibraryError> for Fail {
    fn from(e: LibraryError) -> Self {
        Fail::Error(e)
    }
}

impl Gateway {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cfg: GatewayConfig,
        auth: PgPool,
        app: PgPool,
        sessions: Arc<SessionService>,
        handler: Arc<dyn RpcHandler>,
        audit: Arc<dyn AuditSink>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let verifier = sessions.verifier();
        Self {
            limiter: RateLimiter::new(cfg.limits),
            cfg,
            auth,
            app,
            sessions,
            verifier,
            handler,
            audit,
            hub: Hub::new(),
            clock,
        }
    }

    pub fn hub(&self) -> &Hub {
        &self.hub
    }

    /// Check 1, at connect: the access token is genuine and unexpired, and the device, member and tenant
    /// are still allowed. Everything the connection may do afterwards comes from the context built here.
    pub async fn open(&self, bearer: &str) -> Result<Session, LibraryError> {
        let now = self.clock.now();
        let claims = self.verifier.verify(bearer, now)?;
        let ctx = build_ctx(&self.auth, claims.sub, claims.tid, claims.did).await?;
        Ok(Session {
            ctx,
            access_exp: claims.exp,
            ctx_built_at: now,
        })
    }

    /// Refresh without a connection: a node whose access token expired while it was offline cannot open
    /// the socket, so it exchanges its refresh token over HTTPS. The token is the whole credential; the
    /// network address only rate limits, so one client hammering the endpoint cannot starve the others.
    pub async fn refresh_http(
        &self,
        refresh: &str,
        client: &str,
    ) -> Result<TokenPairWire, LibraryError> {
        self.limiter.check_key(
            &*self.clock,
            &format!("refresh:{client}"),
            crate::Limit {
                burst: 10.0,
                per_second: 0.5,
            },
        )?;
        let now = self.clock.now();
        let pair = self.sessions.refresh(refresh, now).await?;
        Ok(TokenPairWire {
            access: pair.access,
            refresh: pair.refresh,
            access_expires_at: pair.access_expires_at.unix_timestamp(),
        })
    }

    /// One message in, one response out. Never panics on bad input.
    pub async fn handle(&self, s: &mut Session, raw: &str) -> RpcResponse {
        if raw.len() > self.cfg.max_message_bytes {
            return RpcResponse::err(
                0,
                LibraryError::new(ErrorCode::PayloadTooLarge, "That message is too large."),
            );
        }
        let req: RpcRequest = match serde_json::from_str(raw) {
            Ok(r) => r,
            Err(_) => return RpcResponse::parse_error(),
        };
        let id = req.id;
        let started = std::time::Instant::now();
        let method = req.method.clone();
        let result = self.process(s, req).await;
        // Method, ids and outcome only. Params and results never reach the log.
        let outcome = match &result {
            Ok(_) => "ok".to_string(),
            Err(Fail::NoMethod) => "no_method".to_string(),
            Err(Fail::Error(e)) => format!("{:?}", e.code),
        };
        tracing::info!(target: "gateway", %method, tenant = %s.ctx.tenant_id, trace = %s.ctx.trace_id, outcome, ms = started.elapsed().as_millis() as u64, "rpc");
        match result {
            Ok(v) => RpcResponse::ok(id, v),
            Err(Fail::NoMethod) => RpcResponse::method_not_found(id),
            Err(Fail::Error(e)) => RpcResponse::err(id, e),
        }
    }

    async fn process(&self, s: &mut Session, req: RpcRequest) -> Result<Value, Fail> {
        if req.jsonrpc != "2.0" {
            return Err(LibraryError::new(
                ErrorCode::InvalidParams,
                "Only JSON-RPC 2.0 is supported.",
            )
            .into());
        }
        let decl = find(&req.method)
            .filter(|m| m.link == Link::NodeToGateway)
            .ok_or(Fail::NoMethod)?;
        let now = self.clock.now();
        let is_refresh = decl.name == "session.refresh";

        // Check 1 again, per message: an expired access token means refresh first.
        if !is_refresh && now.unix_timestamp() >= s.access_exp {
            return Err(LibraryError::new(
                ErrorCode::TokenExpired,
                "Your session expired. Refreshing it now.",
            )
            .into());
        }
        // Roles, scopes and device revocation take effect within the context's lifetime.
        if (now - s.ctx_built_at).whole_milliseconds() > i128::from(self.cfg.ctx_ttl_ms) {
            self.rebuild(s, now).await?;
        }
        let user = s.ctx.user_id().ok_or_else(unauthenticated)?;

        // Check 2: rate limit. A refused call costs nothing.
        self.limiter
            .check(&*self.clock, s.ctx.tenant_id, user, decl.name)?;

        // Check 3: role, scope and plan, as the method declares them.
        let scope = match req.params.get("scope_id") {
            Some(Value::String(v)) => Some(Uuid::parse_str(v).map(ScopeId).map_err(|_| {
                LibraryError::new(ErrorCode::InvalidParams, "scope_id is not valid.")
            })?),
            Some(_) => {
                return Err(
                    LibraryError::new(ErrorCode::InvalidParams, "scope_id is not valid.").into(),
                );
            }
            None => None,
        };
        authorize(decl, &s.ctx, scope)?;

        match decl.name {
            "session.whoami" => {
                return Ok(serde_json::to_value(WhoAmI {
                    tenant_id: s.ctx.tenant_id,
                    user_id: user,
                    roles: s.ctx.roles.clone(),
                    scopes: s.ctx.scopes.clone(),
                    plan: s.ctx.plan,
                })
                .map_err(|_| LibraryError::internal())?);
            }
            "session.refresh" => {
                return self.refresh(s, &req.params, now).await.map_err(Fail::from);
            }
            _ => {}
        }
        if matches!(decl.access, Access::Server) {
            return Err(LibraryError::new(
                ErrorCode::PermissionDenied,
                "This method is not available to devices.",
            )
            .into());
        }

        // Check 4: idempotency for writes.
        let key = if decl.write {
            let key = req.idempotency_key.as_deref().filter(|k| idempotency::valid_key(k)).ok_or_else(|| LibraryError::new(ErrorCode::InvalidParams, "This call changes something, so it needs an idempotency key of 8 to 128 letters, digits, dashes, dots, colons or underscores."))?;
            match idempotency::claim(
                &self.app,
                &s.ctx,
                key,
                decl.name,
                &req.params,
                self.cfg.pending_timeout_s,
            )
            .await?
            {
                Claim::Replay(stored) => return replay(&stored),
                Claim::Fresh => Some(key.to_string()),
            }
        } else {
            None
        };

        // Every write is on the record before it runs, so no change can happen unaudited.
        if decl.write {
            let record = AuditRecord {
                actor: format!("user:{user}"),
                action: decl.name.to_string(),
                target: scope.map(|s| format!("scope:{s}")),
                tool_hash: None,
                payload_hash: payload_hash(&req.params),
            };
            if let Err(e) = self.audit.record(&s.ctx, record).await {
                if let Some(k) = &key {
                    let _ = idempotency::release(&self.app, &s.ctx, k).await;
                }
                return Err(e.into());
            }
        }

        // Check 5: the handler.
        let outcome = self
            .handler
            .call(
                &s.ctx,
                Call {
                    method: decl.name.to_string(),
                    params: req.params,
                    write: decl.write,
                },
            )
            .await;
        if let Some(k) = key {
            match &outcome {
                Ok(v) => {
                    idempotency::finish(&self.app, &s.ctx, &k, &serde_json::json!({ "ok": v }))
                        .await?
                }
                // A failure worth retrying must not be frozen as the answer.
                Err(e) if e.retryable => idempotency::release(&self.app, &s.ctx, &k).await?,
                Err(e) => {
                    idempotency::finish(&self.app, &s.ctx, &k, &serde_json::json!({ "error": e }))
                        .await?
                }
            }
        }
        outcome.map_err(Fail::from)
    }

    async fn rebuild(&self, s: &mut Session, now: OffsetDateTime) -> Result<(), LibraryError> {
        let (Some(user), Some(device)) = (s.ctx.user_id(), s.ctx.device_id) else {
            return Err(unauthenticated());
        };
        match build_ctx(&self.auth, user, s.ctx.tenant_id, device).await {
            Ok(mut fresh) => {
                fresh.trace_id = s.ctx.trace_id;
                s.ctx = fresh;
                s.ctx_built_at = now;
                Ok(())
            }
            Err(e) => {
                // The member or device lost access: end the connection, not just this call.
                self.hub.disconnect(device);
                Err(e)
            }
        }
    }

    /// `session.refresh`: only this connection's own refresh token works, and the new context is rebuilt
    /// from the database so a refresh also picks up changed roles and scopes. Tokens are never stored
    /// as an idempotency response.
    async fn refresh(
        &self,
        s: &mut Session,
        params: &Value,
        now: OffsetDateTime,
    ) -> Result<Value, LibraryError> {
        let token = params
            .get("refresh")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                LibraryError::new(ErrorCode::InvalidParams, "A refresh token is required.")
            })?;
        let (Some(user), Some(device)) = (s.ctx.user_id(), s.ctx.device_id) else {
            return Err(unauthenticated());
        };
        let pair = self
            .sessions
            .refresh_for(token, Some((user, s.ctx.tenant_id, device)), now)
            .await?;
        let claims = self.verifier.verify(&pair.access, now)?;
        s.access_exp = claims.exp;
        self.rebuild(s, now).await?;
        serde_json::to_value(TokenPairWire {
            access: pair.access,
            refresh: pair.refresh,
            access_expires_at: pair.access_expires_at.unix_timestamp(),
        })
        .map_err(|_| LibraryError::internal())
    }
}

fn replay(stored: &Value) -> Result<Value, Fail> {
    if let Some(v) = stored.get("ok") {
        return Ok(v.clone());
    }
    match stored
        .get("error")
        .and_then(|e| serde_json::from_value::<LibraryError>(e.clone()).ok())
    {
        Some(e) => Err(Fail::Error(e)),
        None => Err(Fail::Error(LibraryError::internal())),
    }
}
