use library_core::{ErrorCode, LibraryError, TenantId};
use sqlx::{PgPool, Row};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::{
    envelope::{self, Sealed},
    keys::MasterKeys,
};

/// Reference to a stored secret. Other tables hold this id (`secret_ref`), never the secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SecretId(pub Uuid);

/// Outcome of a rotation pass over one tenant.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RotateReport {
    pub rewrapped: u64,
    pub failed: u64,
}

/// Reads and writes `secrets` through the `app_vault` role, the only role with access to that table.
pub struct Vault {
    pool: PgPool,
    keys: MasterKeys,
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vault({:?})", self.keys)
    }
}

fn db(_e: sqlx::Error) -> LibraryError {
    LibraryError::internal()
}

fn not_found() -> LibraryError {
    LibraryError::new(ErrorCode::NotFound, "That secret does not exist.")
}

impl Vault {
    pub fn new(pool: PgPool, keys: MasterKeys) -> Self {
        Self { pool, keys }
    }

    async fn begin(
        &self,
        tenant: TenantId,
    ) -> Result<sqlx::Transaction<'static, sqlx::Postgres>, LibraryError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
            .bind(tenant.0.to_string())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        Ok(tx)
    }

    pub async fn put(&self, tenant: TenantId, plaintext: &[u8]) -> Result<SecretId, LibraryError> {
        let id = Uuid::new_v4();
        let sealed = envelope::seal(&self.keys, tenant, id, plaintext)?;
        let mut tx = self.begin(tenant).await?;
        sqlx::query("INSERT INTO secrets (id, tenant_id, ciphertext, wrapped_dek, key_version) VALUES ($1, $2, $3, $4, $5)")
            .bind(id)
            .bind(tenant.0)
            .bind(&sealed.ciphertext)
            .bind(&sealed.wrapped_dek)
            .bind(i32::try_from(sealed.key_version).map_err(|_| LibraryError::internal())?)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(SecretId(id))
    }

    /// Decrypts a secret for the dispatcher or model proxy. The caller must drop it as soon as it is used.
    pub async fn get(
        &self,
        tenant: TenantId,
        id: SecretId,
    ) -> Result<Zeroizing<Vec<u8>>, LibraryError> {
        let mut tx = self.begin(tenant).await?;
        let row = sqlx::query("SELECT ciphertext, wrapped_dek, key_version FROM secrets WHERE id = $1 AND tenant_id = $2")
            .bind(id.0)
            .bind(tenant.0)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(not_found)?;
        tx.rollback().await.map_err(db)?;
        let sealed = Sealed {
            ciphertext: row.get("ciphertext"),
            wrapped_dek: row.get("wrapped_dek"),
            key_version: u32::try_from(row.get::<i32, _>("key_version"))
                .map_err(|_| LibraryError::internal())?,
        };
        envelope::open(&self.keys, tenant, id.0, &sealed)
    }

    /// Replaces a secret's value (for example after a token refresh) under a fresh data key.
    pub async fn replace(
        &self,
        tenant: TenantId,
        id: SecretId,
        plaintext: &[u8],
    ) -> Result<(), LibraryError> {
        let sealed = envelope::seal(&self.keys, tenant, id.0, plaintext)?;
        let mut tx = self.begin(tenant).await?;
        let n = sqlx::query("UPDATE secrets SET ciphertext = $3, wrapped_dek = $4, key_version = $5 WHERE id = $1 AND tenant_id = $2")
            .bind(id.0)
            .bind(tenant.0)
            .bind(&sealed.ciphertext)
            .bind(&sealed.wrapped_dek)
            .bind(i32::try_from(sealed.key_version).map_err(|_| LibraryError::internal())?)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
        if n == 0 {
            return Err(not_found());
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Re-wraps every data key of a tenant that is not on the current master key version.
    /// One secret that cannot be unwrapped (corrupt, or sealed under a key that was already retired)
    /// does not stop the rest; it is counted in `failed` so an operator can alert on it. Old master
    /// keys can be retired only when every tenant reports `failed == 0`.
    pub async fn rotate(&self, tenant: TenantId) -> Result<RotateReport, LibraryError> {
        let current =
            i32::try_from(self.keys.current_version()).map_err(|_| LibraryError::internal())?;
        let mut tx = self.begin(tenant).await?;
        let rows = sqlx::query("SELECT id, ciphertext, wrapped_dek, key_version FROM secrets WHERE tenant_id = $1 AND key_version <> $2 FOR UPDATE")
            .bind(tenant.0)
            .bind(current)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut report = RotateReport::default();
        for r in rows {
            let id: Uuid = r.get("id");
            let old = Sealed {
                ciphertext: r.get("ciphertext"),
                wrapped_dek: r.get("wrapped_dek"),
                key_version: u32::try_from(r.get::<i32, _>("key_version"))
                    .map_err(|_| LibraryError::internal())?,
            };
            let Ok(new) = envelope::rewrap(&self.keys, tenant, id, &old) else {
                report.failed += 1;
                continue;
            };
            sqlx::query("UPDATE secrets SET wrapped_dek = $2, key_version = $3 WHERE id = $1")
                .bind(id)
                .bind(&new.wrapped_dek)
                .bind(current)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            report.rewrapped += 1;
        }
        tx.commit().await.map_err(db)?;
        Ok(report)
    }
}
