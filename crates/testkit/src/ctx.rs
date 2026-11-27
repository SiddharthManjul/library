use sqlx::PgPool;
use uuid::Uuid;

/// The session variables authz sets for each transaction.
#[derive(Debug, Clone, Default)]
pub struct Ctx {
    pub tenant: Option<Uuid>,
    pub user: Option<Uuid>,
    pub scopes: Vec<Uuid>,
}

impl Ctx {
    pub fn none() -> Self {
        Self::default()
    }

    fn scope_list(&self) -> String {
        self.scopes
            .iter()
            .map(Uuid::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }
}

async fn begin<'a>(
    pool: &'a PgPool,
    ctx: &Ctx,
) -> Result<sqlx::Transaction<'a, sqlx::Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT set_config('app.tenant_id', $1, true), set_config('app.user_id', $2, true), set_config('app.scope_ids', $3, true)")
        .bind(ctx.tenant.map(|t| t.to_string()).unwrap_or_default())
        .bind(ctx.user.map(|u| u.to_string()).unwrap_or_default())
        .bind(ctx.scope_list())
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

/// Rows of `table` visible under `ctx` (the transaction is rolled back).
pub async fn count_as(pool: &PgPool, ctx: &Ctx, table: &str) -> Result<i64, sqlx::Error> {
    let mut tx = begin(pool, ctx).await?;
    let n: i64 = sqlx::query_scalar(sql!("SELECT count(*) FROM \"{table}\""))
        .fetch_one(&mut *tx)
        .await?;
    tx.rollback().await?;
    Ok(n)
}

/// Runs one statement under `ctx` and returns rows affected. Always rolled back,
/// so tests never change the fixture.
pub async fn exec_as(pool: &PgPool, ctx: &Ctx, sql: &str) -> Result<u64, sqlx::Error> {
    let mut tx = begin(pool, ctx).await?;
    let r = sqlx::query(sqlx::AssertSqlSafe(sql.to_string()))
        .execute(&mut *tx)
        .await
        .map(|r| r.rows_affected());
    tx.rollback().await?;
    r
}
