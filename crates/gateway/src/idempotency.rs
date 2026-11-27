//! Idempotency keys in the database. A key belongs to one tenant and user and remembers the method and
//! a hash of the params, so reusing a key for a different request is refused.

use library_authz::begin;
use library_core::{ErrorCode, LibraryError, RequestCtx};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

fn db(_e: sqlx::Error) -> LibraryError {
    LibraryError::internal()
}

pub fn valid_key(k: &str) -> bool {
    (8..=128).contains(&k.len())
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
}

pub fn hash_params(v: &serde_json::Value) -> Vec<u8> {
    // serde_json keeps object keys sorted, so equal params always hash equally.
    Sha256::digest(serde_json::to_vec(v).unwrap_or_default()).to_vec()
}

#[derive(Debug)]
pub enum Claim {
    /// First time: the caller runs the handler, then calls `finish` or `release`.
    Fresh,
    /// Already done: return this response without running anything.
    Replay(serde_json::Value),
}

pub async fn claim(
    pool: &PgPool,
    ctx: &RequestCtx,
    key: &str,
    method: &str,
    params: &serde_json::Value,
    pending_timeout_s: i64,
) -> Result<Claim, LibraryError> {
    let (Some(user), hash) = (ctx.user_id(), hash_params(params)) else {
        return Err(LibraryError::internal());
    };
    let mut tx = begin(pool, ctx).await?;
    let inserted = sqlx::query("INSERT INTO idempotency_keys (tenant_id, user_id, key, method, params_hash) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING")
        .bind(ctx.tenant_id.0)
        .bind(user.0)
        .bind(key)
        .bind(method)
        .bind(&hash)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
    if inserted == 1 {
        tx.commit().await.map_err(db)?;
        return Ok(Claim::Fresh);
    }
    let row = sqlx::query("SELECT method, params_hash, state, response, (created_at < now() - make_interval(secs => $3::double precision)) AS stale FROM idempotency_keys WHERE tenant_id = $1 AND key = $2 FOR UPDATE")
        .bind(ctx.tenant_id.0)
        .bind(key)
        .bind(pending_timeout_s as f64)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
    let (m, h, state): (String, Vec<u8>, String) =
        (row.get("method"), row.get("params_hash"), row.get("state"));
    if m != method || h != hash {
        return Err(LibraryError::new(
            ErrorCode::Conflict,
            "That idempotency key was already used for a different request.",
        ));
    }
    if state == "done" {
        let response: Option<serde_json::Value> = row.get("response");
        return response
            .map(Claim::Replay)
            .ok_or_else(LibraryError::internal);
    }
    // Pending: another call is running it, unless that call died long ago.
    if row.get::<bool, _>("stale") {
        sqlx::query(
            "UPDATE idempotency_keys SET created_at = now() WHERE tenant_id = $1 AND key = $2",
        )
        .bind(ctx.tenant_id.0)
        .bind(key)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        return Ok(Claim::Fresh);
    }
    Err(LibraryError::new(
        ErrorCode::Conflict,
        "This request is still being processed. Try again in a moment.",
    ))
}

/// Remembers the first result so a replay returns it.
pub async fn finish(
    pool: &PgPool,
    ctx: &RequestCtx,
    key: &str,
    response: &serde_json::Value,
) -> Result<(), LibraryError> {
    let mut tx = begin(pool, ctx).await?;
    sqlx::query("UPDATE idempotency_keys SET state = 'done', response = $3 WHERE tenant_id = $1 AND key = $2").bind(ctx.tenant_id.0).bind(key).bind(response).execute(&mut *tx).await.map_err(db)?;
    tx.commit().await.map_err(db)
}

/// Forgets a key whose call failed in a way worth retrying.
pub async fn release(pool: &PgPool, ctx: &RequestCtx, key: &str) -> Result<(), LibraryError> {
    let mut tx = begin(pool, ctx).await?;
    sqlx::query(
        "DELETE FROM idempotency_keys WHERE tenant_id = $1 AND key = $2 AND state = 'pending'",
    )
    .bind(ctx.tenant_id.0)
    .bind(key)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    tx.commit().await.map_err(db)
}
