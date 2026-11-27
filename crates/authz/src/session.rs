//! Refresh-token sessions: rotation with reuse detection.
//!
//! A refresh token is 32 random bytes; only its SHA-256 hash is stored. Every use rotates it: the old
//! row is marked used and a new one is issued in the same family. Presenting a token that was already
//! used means it leaked or was replayed, so the whole family is revoked and the caller must sign in again.
//! Runs as `app_auth`, the one role allowed to read sign-in tables before a tenant context exists.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use library_core::{DeviceId, ErrorCode, LibraryError, TenantId, UserId};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::token::{ACCESS_TTL_SECS, TokenSigner};

/// Refresh tokens live 30 days from issue; rotation does not extend the family.
pub const REFRESH_TTL_DAYS: i64 = 30;
const PREFIX: &str = "lrt1_";

#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access: String,
    pub access_expires_at: OffsetDateTime,
    pub refresh: String,
}

pub struct SessionService {
    auth: PgPool,
    signer: TokenSigner,
}

fn db_err(e: sqlx::Error) -> LibraryError {
    // The cause is logged by the caller's tracing layer; nothing from the database reaches the client.
    let _ = e;
    LibraryError::internal()
}

fn unauthenticated() -> LibraryError {
    LibraryError::new(ErrorCode::Unauthenticated, "Sign in again to continue.")
}

fn new_refresh_secret() -> Result<(String, Vec<u8>), LibraryError> {
    let mut raw = [0u8; 32];
    getrandom::fill(&mut raw).map_err(|_| LibraryError::internal())?;
    let token = format!("{PREFIX}{}", B64.encode(raw));
    let hash = hash_token(&token);
    Ok((token, hash))
}

fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

impl SessionService {
    pub fn new(auth: PgPool, signer: TokenSigner) -> Self {
        Self { auth, signer }
    }

    pub fn verifier(&self) -> crate::token::TokenVerifier {
        self.signer.verifier()
    }

    /// Starts a session for a device that just signed in, in a new token family.
    pub async fn start(
        &self,
        user: UserId,
        tenant: TenantId,
        device: DeviceId,
        now: OffsetDateTime,
    ) -> Result<TokenPair, LibraryError> {
        let mut tx = self.auth.begin().await.map_err(db_err)?;
        self.ensure_allowed(&mut tx, user, tenant, device).await?;
        let pair = self
            .issue(
                &mut tx,
                user,
                tenant,
                device,
                Uuid::new_v4(),
                now,
                now + Duration::days(REFRESH_TTL_DAYS),
            )
            .await?;
        tx.commit().await.map_err(db_err)?;
        Ok(pair)
    }

    /// Exchanges a refresh token for a new pair, or fails. Reuse revokes the whole family.
    pub async fn refresh(
        &self,
        token: &str,
        now: OffsetDateTime,
    ) -> Result<TokenPair, LibraryError> {
        self.refresh_for(token, None, now).await
    }

    /// Like `refresh`, but when `expected` is given the token must belong to exactly that user, tenant
    /// and device. A mismatch fails without touching the token, so a connection cannot use up (or learn
    /// anything about) another device's session.
    pub async fn refresh_for(
        &self,
        token: &str,
        expected: Option<(UserId, TenantId, DeviceId)>,
        now: OffsetDateTime,
    ) -> Result<TokenPair, LibraryError> {
        if !token.starts_with(PREFIX) {
            return Err(unauthenticated());
        }
        let hash = hash_token(token);
        let mut tx = self.auth.begin().await.map_err(db_err)?;
        let row = sqlx::query("SELECT id, tenant_id, user_id, device_id, family_id, expires_at, used_at, revoked_at FROM refresh_tokens WHERE token_hash = $1 FOR UPDATE")
            .bind(&hash)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
        let Some(row) = row else {
            return Err(unauthenticated());
        };
        let (id, tenant, user, device, family): (Uuid, Uuid, Uuid, Uuid, Uuid) = (
            row.get("id"),
            row.get("tenant_id"),
            row.get("user_id"),
            row.get("device_id"),
            row.get("family_id"),
        );
        let (expires_at, used_at, revoked_at): (
            OffsetDateTime,
            Option<OffsetDateTime>,
            Option<OffsetDateTime>,
        ) = (
            row.get("expires_at"),
            row.get("used_at"),
            row.get("revoked_at"),
        );

        if let Some((u, t, d)) = expected
            && (u.0 != user || t.0 != tenant || d.0 != device)
        {
            return Err(unauthenticated());
        }
        if revoked_at.is_some() {
            return Err(unauthenticated());
        }
        if used_at.is_some() {
            // A used token came back: assume theft and end every session in this family.
            sqlx::query("UPDATE refresh_tokens SET revoked_at = $2 WHERE family_id = $1 AND revoked_at IS NULL")
                .bind(family)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            tx.commit().await.map_err(db_err)?;
            return Err(unauthenticated());
        }
        if now >= expires_at {
            return Err(LibraryError::new(
                ErrorCode::TokenExpired,
                "Your session ended. Sign in again.",
            ));
        }
        let (user, tenant, device) = (UserId(user), TenantId(tenant), DeviceId(device));
        self.ensure_allowed(&mut tx, user, tenant, device).await?;
        sqlx::query("UPDATE refresh_tokens SET used_at = $2 WHERE id = $1")
            .bind(id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        let pair = self
            .issue(&mut tx, user, tenant, device, family, now, expires_at)
            .await?;
        tx.commit().await.map_err(db_err)?;
        Ok(pair)
    }

    /// Ends every session of one device (sign out, or the device was revoked).
    pub async fn revoke_device(
        &self,
        device: DeviceId,
        now: OffsetDateTime,
    ) -> Result<(), LibraryError> {
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = $2 WHERE device_id = $1 AND revoked_at IS NULL",
        )
        .bind(device.0)
        .bind(now)
        .execute(&self.auth)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    /// The user must still belong to the tenant and the device must not be revoked.
    async fn ensure_allowed(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user: UserId,
        tenant: TenantId,
        device: DeviceId,
    ) -> Result<(), LibraryError> {
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM memberships m WHERE m.tenant_id = $2 AND m.user_id = $1)
                AND EXISTS (SELECT 1 FROM devices d WHERE d.id = $3 AND d.tenant_id = $2 AND d.user_id = $1 AND d.revoked_at IS NULL)",
        )
        .bind(user.0)
        .bind(tenant.0)
        .bind(device.0)
        .fetch_one(&mut **tx)
        .await
        .map_err(db_err)?;
        if ok { Ok(()) } else { Err(unauthenticated()) }
    }

    #[allow(clippy::too_many_arguments)]
    async fn issue(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user: UserId,
        tenant: TenantId,
        device: DeviceId,
        family: Uuid,
        now: OffsetDateTime,
        expires_at: OffsetDateTime,
    ) -> Result<TokenPair, LibraryError> {
        let (refresh, hash) = new_refresh_secret()?;
        sqlx::query("INSERT INTO refresh_tokens (tenant_id, user_id, device_id, family_id, token_hash, issued_at, expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(tenant.0)
            .bind(user.0)
            .bind(device.0)
            .bind(family)
            .bind(hash)
            .bind(now)
            .bind(expires_at)
            .execute(&mut **tx)
            .await
            .map_err(db_err)?;
        let (access, claims) = self.signer.issue(user, tenant, device, now)?;
        debug_assert_eq!(claims.exp - claims.iat, ACCESS_TTL_SECS);
        Ok(TokenPair {
            access,
            access_expires_at: OffsetDateTime::from_unix_timestamp(claims.exp).unwrap_or(now),
            refresh,
        })
    }
}
