//! Boundary traits. Crates talk to each other only through these, never through each other's tables.

use std::{future::Future, pin::Pin};

use crate::{RequestCtx, Result};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// What a handler receives for one call that passed the gateway's checks.
#[derive(Debug, Clone)]
pub struct Call {
    pub method: String,
    pub params: serde_json::Value,
    /// True when the method writes; the idempotency key has already been claimed.
    pub write: bool,
}

/// Implemented by the crates that own methods (work, registry, connections and so on). The gateway has
/// already checked role, plan, scope, rate limit and idempotency before it calls this.
pub trait RpcHandler: Send + Sync {
    fn call<'a>(
        &'a self,
        ctx: &'a RequestCtx,
        call: Call,
    ) -> BoxFuture<'a, Result<serde_json::Value>>;
}

/// One thing worth remembering forever. The details are hashed, never stored, so the audit log holds
/// no secrets and no personal data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    pub actor: String,
    pub action: String,
    pub target: Option<String>,
    pub tool_hash: Option<[u8; 32]>,
    pub payload_hash: [u8; 32],
}

/// Implemented by the audit crate. The gateway records every successful write through it.
pub trait AuditSink: Send + Sync {
    fn record<'a>(&'a self, ctx: &'a RequestCtx, record: AuditRecord) -> BoxFuture<'a, Result<()>>;
}
