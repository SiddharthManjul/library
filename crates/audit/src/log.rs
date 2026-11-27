use library_core::{LibraryError, RequestCtx};
use sqlx::{PgPool, Row as _};
use time::OffsetDateTime;

use crate::canonical::{AuditEntry, Row, genesis_hash, micros, row_hash};

/// Appends to and reads the audit log through the runtime role. Appends are serialised per tenant by a
/// transaction-scoped advisory lock, so sequence numbers have no gaps and no two rows share a predecessor.
pub struct AuditLog {
    pool: PgPool,
}

fn db(_e: sqlx::Error) -> LibraryError {
    LibraryError::internal()
}

fn to_hash(v: Vec<u8>) -> Result<[u8; 32], LibraryError> {
    v.try_into().map_err(|_| LibraryError::internal())
}

impl AuditLog {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Appends one row to the caller's tenant and returns its sequence number.
    pub async fn append(&self, ctx: &RequestCtx, entry: &AuditEntry) -> Result<i64, LibraryError> {
        self.append_at(ctx, entry, OffsetDateTime::now_utc()).await
    }

    pub async fn append_at(
        &self,
        ctx: &RequestCtx,
        entry: &AuditEntry,
        at: OffsetDateTime,
    ) -> Result<i64, LibraryError> {
        let tenant = ctx.tenant_id;
        let mut tx = library_authz::begin(&self.pool, ctx).await?;
        // One writer per tenant at a time. The lock ends with the transaction.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("audit:{tenant}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let last = sqlx::query(
            "SELECT seq, hash FROM audit_log WHERE tenant_id = $1 ORDER BY seq DESC LIMIT 1",
        )
        .bind(tenant.0)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (seq, prev) = match last {
            Some(r) => (r.get::<i64, _>("seq") + 1, to_hash(r.get("hash"))?),
            None => (1, genesis_hash(tenant)),
        };
        let created_us = micros(at);
        let hash = row_hash(tenant, &prev, seq, created_us, entry);
        sqlx::query(
            "INSERT INTO audit_log (tenant_id, seq, actor, action, target, tool_hash, payload_hash, prev_hash, hash, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, to_timestamp($10::double precision / 1000000))",
        )
        .bind(tenant.0)
        .bind(seq)
        .bind(&entry.actor)
        .bind(&entry.action)
        .bind(&entry.target)
        .bind(entry.tool_hash.as_ref().map(<[u8; 32]>::as_slice))
        .bind(entry.payload_hash.as_slice())
        .bind(prev.as_slice())
        .bind(hash.as_slice())
        .bind(created_us as f64)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(seq)
    }

    /// Every row of a tenant in order, for the verifier.
    pub async fn read_all(&self, ctx: &RequestCtx) -> Result<Vec<Row>, LibraryError> {
        let mut tx = library_authz::begin(&self.pool, ctx).await?;
        let rows = sqlx::query(
            "SELECT seq, actor, action, target, tool_hash, payload_hash, prev_hash, hash,
                    (extract(epoch from created_at) * 1000000)::bigint AS created_us
             FROM audit_log WHERE tenant_id = $1 ORDER BY seq",
        )
        .bind(ctx.tenant_id.0)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.rollback().await.map_err(db)?;
        rows.into_iter()
            .map(|r| {
                Ok(Row {
                    seq: r.get("seq"),
                    created_us: r.get("created_us"),
                    prev_hash: to_hash(r.get("prev_hash"))?,
                    hash: to_hash(r.get("hash"))?,
                    entry: AuditEntry {
                        actor: r.get("actor"),
                        action: r.get("action"),
                        target: r.get("target"),
                        tool_hash: r
                            .get::<Option<Vec<u8>>, _>("tool_hash")
                            .map(to_hash)
                            .transpose()?,
                        payload_hash: to_hash(r.get("payload_hash"))?,
                    },
                })
            })
            .collect()
    }
}

impl library_core::AuditSink for AuditLog {
    fn record<'a>(
        &'a self,
        ctx: &'a RequestCtx,
        r: library_core::AuditRecord,
    ) -> library_core::BoxFuture<'a, Result<(), LibraryError>> {
        Box::pin(async move {
            let entry = AuditEntry {
                actor: r.actor,
                action: r.action,
                target: r.target,
                tool_hash: r.tool_hash,
                payload_hash: r.payload_hash,
            };
            self.append(ctx, &entry).await.map(|_| ())
        })
    }
}
