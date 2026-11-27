use library_core::{ErrorCode, LibraryError, RequestCtx};
use serde_json::Value;
use uuid::Uuid;

use crate::redact::scrub_json;

/// What the log is for: every routing, decomposition, approval, lease, relay and change-detection decision,
/// with its inputs, so the v1 decision API can be trained on real data. Closed on purpose (the database
/// refuses any other kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionKind {
    Routing,
    Decomposition,
    ApprovalTier,
    Lease,
    AdvisorCall,
    RelayAssignment,
    ChangeTrigger,
}

impl DecisionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DecisionKind::Routing => "routing",
            DecisionKind::Decomposition => "decomposition",
            DecisionKind::ApprovalTier => "approval_tier",
            DecisionKind::Lease => "lease",
            DecisionKind::AdvisorCall => "advisor_call",
            DecisionKind::RelayAssignment => "relay_assignment",
            DecisionKind::ChangeTrigger => "change_trigger",
        }
    }
}

const MAX_INPUT_BYTES: usize = 16 * 1024;

pub struct DecisionLog {
    pool: sqlx::PgPool,
}

impl DecisionLog {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    /// Appends one decision for the caller's tenant. Inputs are scrubbed of anything credential-shaped
    /// before they are stored, and are limited to 16 KB: this is a record of why, not a copy of the data.
    pub async fn record(
        &self,
        ctx: &RequestCtx,
        run_id: Option<Uuid>,
        kind: DecisionKind,
        inputs: &Value,
        choice: &str,
        outcome: Option<&str>,
    ) -> Result<Uuid, LibraryError> {
        let inputs = scrub_json(inputs);
        if inputs.to_string().len() > MAX_INPUT_BYTES {
            return Err(LibraryError::new(
                ErrorCode::PayloadTooLarge,
                "Decision inputs are too large.",
            ));
        }
        let mut tx = library_authz::begin(&self.pool, ctx).await?;
        let id: Uuid = sqlx::query_scalar("INSERT INTO decision_log (tenant_id, run_id, kind, inputs, choice, outcome) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id")
            .bind(ctx.tenant_id.0)
            .bind(run_id)
            .bind(kind.as_str())
            .bind(inputs)
            .bind(crate::redact::scrub(choice).as_ref())
            .bind(outcome.map(|o| crate::redact::scrub(o).into_owned()))
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| LibraryError::internal())?;
        tx.commit().await.map_err(|_| LibraryError::internal())?;
        Ok(id)
    }
}
