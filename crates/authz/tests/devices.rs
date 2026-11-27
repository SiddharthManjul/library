//! Device registration tests (P0-11).
#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not

use ed25519_dalek::{Signer, SigningKey};
use library_authz::devices::{DeviceRegistration, register};
use library_core::{ErrorCode, TenantId, UserId, device_registration_message};
use library_testkit::{TestDb, seed_two_tenants};
use time::OffsetDateTime;

struct Dev {
    signing: SigningKey,
    x25519: [u8; 32],
}

fn dev(seed: u8) -> Dev {
    Dev {
        signing: SigningKey::from_bytes(&[seed; 32]),
        x25519: [seed.wrapping_add(100); 32],
    }
}

fn reg(d: &Dev, tenant: TenantId, user: UserId) -> DeviceRegistration {
    DeviceRegistration {
        x25519_public: d.x25519,
        ed25519_public: d.signing.verifying_key().to_bytes(),
        proof: d
            .signing
            .sign(&device_registration_message(tenant, user, &d.x25519))
            .to_bytes(),
    }
}

#[tokio::test]
async fn registers_and_is_idempotent() -> anyhow::Result<()> {
    let db = TestDb::new().await?;
    let fx = seed_two_tenants(&db).await?;
    let (t, u) = (TenantId(fx.a.id), UserId(fx.a.user));
    let d = dev(1);
    let first = register(&db.auth, t, u, &reg(&d, t, u), OffsetDateTime::now_utc()).await?;
    let again = register(&db.auth, t, u, &reg(&d, t, u), OffsetDateTime::now_utc()).await?;
    assert_eq!(
        first, again,
        "registering the same keys twice returns the same device"
    );
    let stored: (Vec<u8>, Vec<u8>) =
        sqlx::query_as("SELECT public_key, signing_public_key FROM devices WHERE id = $1")
            .bind(first.0)
            .fetch_one(&db.admin)
            .await?;
    assert_eq!(stored.0, d.x25519.to_vec());
    assert_eq!(stored.1, d.signing.verifying_key().to_bytes().to_vec());
    db.drop_db().await
}

#[tokio::test]
async fn a_bad_or_replayed_proof_is_rejected() -> anyhow::Result<()> {
    let db = TestDb::new().await?;
    let fx = seed_two_tenants(&db).await?;
    let (ta, ua) = (TenantId(fx.a.id), UserId(fx.a.user));
    let d = dev(2);
    // Proof made for tenant B replayed against tenant A.
    let (tb, ub) = (TenantId(fx.b.id), UserId(fx.b.user));
    let mut replay = reg(&d, tb, ub);
    assert_eq!(
        register(&db.auth, ta, ua, &replay, OffsetDateTime::now_utc())
            .await
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    );
    // Flipped signature byte.
    replay = reg(&d, ta, ua);
    replay.proof[0] ^= 1;
    assert!(
        register(&db.auth, ta, ua, &replay, OffsetDateTime::now_utc())
            .await
            .is_err()
    );
    // Someone else's sealing key under our signature.
    let mut swapped = reg(&d, ta, ua);
    swapped.x25519_public = [9; 32];
    assert!(
        register(&db.auth, ta, ua, &swapped, OffsetDateTime::now_utc())
            .await
            .is_err()
    );
    // Proof signed by a different key than the one presented.
    let mut other = reg(&d, ta, ua);
    other.ed25519_public = dev(3).signing.verifying_key().to_bytes();
    assert!(
        register(&db.auth, ta, ua, &other, OffsetDateTime::now_utc())
            .await
            .is_err()
    );
    db.drop_db().await
}

#[tokio::test]
async fn only_members_can_register() -> anyhow::Result<()> {
    let db = TestDb::new().await?;
    let fx = seed_two_tenants(&db).await?;
    let d = dev(4);
    // User of tenant B is not a member of tenant A.
    let (t, u) = (TenantId(fx.a.id), UserId(fx.b.user));
    assert!(
        register(&db.auth, t, u, &reg(&d, t, u), OffsetDateTime::now_utc())
            .await
            .is_err()
    );
    db.drop_db().await
}

#[tokio::test]
async fn nobody_can_take_over_or_revive_a_device() -> anyhow::Result<()> {
    let db = TestDb::new().await?;
    let fx = seed_two_tenants(&db).await?;
    let (t, u) = (TenantId(fx.a.id), UserId(fx.a.user));
    let d = dev(5);
    let id = register(&db.auth, t, u, &reg(&d, t, u), OffsetDateTime::now_utc()).await?;

    // A second member of the same tenant presents the same sealing key with their own signing key.
    let intruder_user: uuid::Uuid = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, display_name) VALUES ($1, 'x@example.test', 'x')")
        .bind(intruder_user)
        .execute(&db.admin)
        .await?;
    sqlx::query("INSERT INTO memberships (tenant_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(t.0)
        .bind(intruder_user)
        .execute(&db.admin)
        .await?;
    let iu = UserId(intruder_user);
    let intruder = Dev {
        signing: SigningKey::from_bytes(&[55; 32]),
        x25519: d.x25519,
    };
    assert!(
        register(
            &db.auth,
            t,
            iu,
            &reg(&intruder, t, iu),
            OffsetDateTime::now_utc()
        )
        .await
        .is_err(),
        "same sealing key, other user"
    );

    // A revoked device cannot register itself back to life.
    sqlx::query("UPDATE devices SET revoked_at = now() WHERE id = $1")
        .bind(id.0)
        .execute(&db.admin)
        .await?;
    assert!(
        register(&db.auth, t, u, &reg(&d, t, u), OffsetDateTime::now_utc())
            .await
            .is_err()
    );
    let revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM devices WHERE id = $1")
            .bind(id.0)
            .fetch_one(&db.admin)
            .await?;
    assert!(revoked);
    db.drop_db().await
}
