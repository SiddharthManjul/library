//! Device registration (P0-11). A device proves it holds its Ed25519 key by signing a message bound to
//! the tenant, the user and its X25519 sealing key; the server then records both public keys.
//! Private keys never reach the server.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use library_core::{
    DeviceId, ErrorCode, LibraryError, TenantId, UserId, device_registration_message,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

pub use library_core::DeviceRegistration;

fn denied() -> LibraryError {
    LibraryError::new(
        ErrorCode::PermissionDenied,
        "This device cannot be registered.",
    )
}

/// Registers a device, or confirms an existing registration of the same keys by the same user.
/// A revoked device stays revoked, and nobody can take over another user's keys.
pub async fn register(
    auth: &PgPool,
    tenant: TenantId,
    user: UserId,
    reg: &DeviceRegistration,
    now: OffsetDateTime,
) -> Result<DeviceId, LibraryError> {
    let key = VerifyingKey::from_bytes(&reg.ed25519_public).map_err(|_| denied())?;
    key.verify(
        &device_registration_message(tenant, user, &reg.x25519_public),
        &Signature::from_bytes(&reg.proof),
    )
    .map_err(|_| denied())?;

    let is_member: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM memberships WHERE tenant_id = $1 AND user_id = $2)",
    )
    .bind(tenant.0)
    .bind(user.0)
    .fetch_one(auth)
    .await
    .map_err(|_| LibraryError::internal())?;
    if !is_member {
        return Err(denied());
    }

    let id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO devices (tenant_id, user_id, public_key, signing_public_key, last_seen)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (tenant_id, public_key) DO UPDATE SET last_seen = EXCLUDED.last_seen
           WHERE devices.user_id = EXCLUDED.user_id
             AND devices.signing_public_key = EXCLUDED.signing_public_key
             AND devices.revoked_at IS NULL
         RETURNING id",
    )
    .bind(tenant.0)
    .bind(user.0)
    .bind(reg.x25519_public.as_slice())
    .bind(reg.ed25519_public.as_slice())
    .bind(now)
    .fetch_optional(auth)
    .await
    .map_err(|_| LibraryError::internal())?;
    id.map(DeviceId).ok_or_else(denied)
}
