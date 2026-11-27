//! Every database call made for a request starts here. The transaction carries the tenant, user and
//! scope ids as `SET LOCAL` variables, so row-level security applies to everything inside it, and
//! the variables disappear when the transaction ends (they never leak to the next user of the connection).

use library_core::{LibraryError, RequestCtx};
use sqlx::{PgPool, Postgres, Transaction};

/// Begins a transaction on the runtime role with the request's tenant, user and scopes set.
pub async fn begin(
    pool: &PgPool,
    ctx: &RequestCtx,
) -> Result<Transaction<'static, Postgres>, LibraryError> {
    let mut tx = pool.begin().await.map_err(|_| LibraryError::internal())?;
    let scopes = ctx
        .scopes
        .iter()
        .map(|s| s.0.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let user = ctx.user_id().map(|u| u.0.to_string()).unwrap_or_default();
    sqlx::query("SELECT set_config('app.tenant_id', $1, true), set_config('app.user_id', $2, true), set_config('app.scope_ids', $3, true), set_config('app.trace_id', $4, true)")
        .bind(ctx.tenant_id.0.to_string())
        .bind(user)
        .bind(scopes)
        .bind(ctx.trace_id.0.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|_| LibraryError::internal())?;
    Ok(tx)
}
