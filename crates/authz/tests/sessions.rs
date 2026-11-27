//! Session tests (P0-10): token rotation, reuse detection and revocation against real Postgres.
#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not

use library_authz::{SessionService, TokenSigner, session::REFRESH_TTL_DAYS};
use library_core::{DeviceId, ErrorCode, TenantId, UserId};
use library_testkit::{Fixture, TestDb, seed_two_tenants};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

struct Env {
    db: TestDb,
    fx: Fixture,
    svc: SessionService,
}

async fn env() -> anyhow::Result<Env> {
    let db = TestDb::new().await?;
    let fx = seed_two_tenants(&db).await?;
    let svc = SessionService::new(db.auth.clone(), TokenSigner::generate()?);
    Ok(Env { db, fx, svc })
}

async fn device_of(db: &TestDb, tenant: Uuid) -> anyhow::Result<DeviceId> {
    let id: Uuid = sqlx::query_scalar("SELECT id FROM devices WHERE tenant_id = $1")
        .bind(tenant)
        .fetch_one(&db.admin)
        .await?;
    Ok(DeviceId(id))
}

fn a(e: &Env) -> (UserId, TenantId) {
    (UserId(e.fx.a.user), TenantId(e.fx.a.id))
}

#[tokio::test]
async fn start_issues_a_verifiable_access_token() -> anyhow::Result<()> {
    let e = env().await?;
    let (user, tenant) = a(&e);
    let device = device_of(&e.db, e.fx.a.id).await?;
    let now = OffsetDateTime::now_utc();
    let pair = e.svc.start(user, tenant, device, now).await?;
    let claims = e.svc.verifier().verify(&pair.access, now)?;
    assert_eq!((claims.sub, claims.tid, claims.did), (user, tenant, device));
    assert!(pair.refresh.starts_with("lrt1_"));
    e.db.drop_db().await
}

#[tokio::test]
async fn refresh_rotates_and_reuse_revokes_the_family() -> anyhow::Result<()> {
    let e = env().await?;
    let (user, tenant) = a(&e);
    let device = device_of(&e.db, e.fx.a.id).await?;
    let now = OffsetDateTime::now_utc();
    let first = e.svc.start(user, tenant, device, now).await?;
    let second = e.svc.refresh(&first.refresh, now).await?;
    assert_ne!(first.refresh, second.refresh);
    let third = e.svc.refresh(&second.refresh, now).await?;

    // Replaying the first (already used) token is theft: the whole family dies, including the newest token.
    let replay = e.svc.refresh(&first.refresh, now).await.unwrap_err();
    assert_eq!(replay.code, ErrorCode::Unauthenticated);
    let after = e.svc.refresh(&third.refresh, now).await.unwrap_err();
    assert_eq!(
        after.code,
        ErrorCode::Unauthenticated,
        "newest token must die with its family"
    );
    e.db.drop_db().await
}

#[tokio::test]
async fn families_are_independent() -> anyhow::Result<()> {
    let e = env().await?;
    let (user, tenant) = a(&e);
    let device = device_of(&e.db, e.fx.a.id).await?;
    let now = OffsetDateTime::now_utc();
    let one = e.svc.start(user, tenant, device, now).await?;
    let two = e.svc.start(user, tenant, device, now).await?;
    let used = e.svc.refresh(&one.refresh, now).await?;
    let _ = e.svc.refresh(&one.refresh, now).await.unwrap_err(); // reuse kills family one
    assert!(e.svc.refresh(&used.refresh, now).await.is_err());
    assert!(
        e.svc.refresh(&two.refresh, now).await.is_ok(),
        "another sign-in must be unaffected"
    );
    e.db.drop_db().await
}

#[tokio::test]
async fn refresh_token_expires_after_30_days_and_rotation_does_not_extend_it() -> anyhow::Result<()>
{
    let e = env().await?;
    let (user, tenant) = a(&e);
    let device = device_of(&e.db, e.fx.a.id).await?;
    let t0 = OffsetDateTime::now_utc();
    let first = e.svc.start(user, tenant, device, t0).await?;
    let day29 = t0 + Duration::days(REFRESH_TTL_DAYS - 1);
    let rotated = e.svc.refresh(&first.refresh, day29).await?;
    let day31 = t0 + Duration::days(REFRESH_TTL_DAYS + 1);
    let err = e.svc.refresh(&rotated.refresh, day31).await.unwrap_err();
    assert_eq!(err.code, ErrorCode::TokenExpired);
    e.db.drop_db().await
}

#[tokio::test]
async fn revoked_device_and_removed_member_cannot_refresh() -> anyhow::Result<()> {
    let e = env().await?;
    let (user, tenant) = a(&e);
    let device = device_of(&e.db, e.fx.a.id).await?;
    let now = OffsetDateTime::now_utc();
    let pair = e.svc.start(user, tenant, device, now).await?;

    sqlx::query("UPDATE devices SET revoked_at = now() WHERE id = $1")
        .bind(device.0)
        .execute(&e.db.admin)
        .await?;
    assert_eq!(
        e.svc.refresh(&pair.refresh, now).await.unwrap_err().code,
        ErrorCode::Unauthenticated
    );
    assert_eq!(
        e.svc
            .start(user, tenant, device, now)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unauthenticated
    );

    // A different device, but the member leaves the tenant.
    let other = e.fx.b.id;
    let device_b = device_of(&e.db, other).await?;
    let pair_b = e
        .svc
        .start(UserId(e.fx.b.user), TenantId(other), device_b, now)
        .await?;
    sqlx::query("DELETE FROM memberships WHERE tenant_id = $1")
        .bind(other)
        .execute(&e.db.admin)
        .await?;
    assert_eq!(
        e.svc.refresh(&pair_b.refresh, now).await.unwrap_err().code,
        ErrorCode::Unauthenticated
    );
    e.db.drop_db().await
}

#[tokio::test]
async fn a_device_of_another_tenant_or_user_cannot_start() -> anyhow::Result<()> {
    let e = env().await?;
    let (user, tenant) = a(&e);
    let device_b = device_of(&e.db, e.fx.b.id).await?;
    let now = OffsetDateTime::now_utc();
    assert!(
        e.svc.start(user, tenant, device_b, now).await.is_err(),
        "tenant B's device must not open a tenant A session"
    );
    assert!(
        e.svc
            .start(
                UserId(e.fx.b.user),
                tenant,
                device_of(&e.db, e.fx.a.id).await?,
                now
            )
            .await
            .is_err(),
        "another user's device must not work"
    );
    assert!(
        e.svc
            .start(
                user,
                TenantId::new(),
                device_of(&e.db, e.fx.a.id).await?,
                now
            )
            .await
            .is_err(),
        "unknown tenant"
    );
    e.db.drop_db().await
}

#[tokio::test]
async fn garbage_tokens_are_rejected() -> anyhow::Result<()> {
    let e = env().await?;
    for junk in ["", "lrt1_", "lrt1_AAAA", "not-a-token", "Bearer lrt1_x"] {
        assert_eq!(
            e.svc
                .refresh(junk, OffsetDateTime::now_utc())
                .await
                .unwrap_err()
                .code,
            ErrorCode::Unauthenticated,
            "{junk}"
        );
    }
    e.db.drop_db().await
}

#[tokio::test]
async fn only_a_hash_of_the_refresh_token_is_stored() -> anyhow::Result<()> {
    let e = env().await?;
    let (user, tenant) = a(&e);
    let device = device_of(&e.db, e.fx.a.id).await?;
    let pair = e
        .svc
        .start(user, tenant, device, OffsetDateTime::now_utc())
        .await?;
    let secret = pair.refresh.trim_start_matches("lrt1_").to_string();
    let hits: i64 = sqlx::query_scalar("SELECT count(*) FROM refresh_tokens WHERE position(convert_to($1, 'UTF8') in token_hash) > 0 OR encode(token_hash, 'base64') LIKE '%' || $1 || '%'")
        .bind(&secret)
        .fetch_one(&e.db.admin)
        .await?;
    assert_eq!(hits, 0);
    let len: i32 = sqlx::query_scalar("SELECT length(token_hash) FROM refresh_tokens WHERE token_hash <> '\\x00' ORDER BY issued_at DESC LIMIT 1").fetch_one(&e.db.admin).await?;
    assert_eq!(len, 32, "stored value is a SHA-256 digest");
    e.db.drop_db().await
}

#[tokio::test]
async fn two_simultaneous_refreshes_never_both_win() -> anyhow::Result<()> {
    let e = env().await?;
    let (user, tenant) = a(&e);
    let device = device_of(&e.db, e.fx.a.id).await?;
    let now = OffsetDateTime::now_utc();
    let pair = e.svc.start(user, tenant, device, now).await?;
    let (r1, r2) = tokio::join!(
        e.svc.refresh(&pair.refresh, now),
        e.svc.refresh(&pair.refresh, now)
    );
    assert_eq!(
        [r1.is_ok(), r2.is_ok()].iter().filter(|x| **x).count(),
        1,
        "exactly one concurrent refresh may succeed"
    );
    e.db.drop_db().await
}

#[tokio::test]
async fn revoking_a_device_ends_its_sessions() -> anyhow::Result<()> {
    let e = env().await?;
    let (user, tenant) = a(&e);
    let device = device_of(&e.db, e.fx.a.id).await?;
    let now = OffsetDateTime::now_utc();
    let pair = e.svc.start(user, tenant, device, now).await?;
    e.svc.revoke_device(device, now).await?;
    assert!(e.svc.refresh(&pair.refresh, now).await.is_err());
    e.db.drop_db().await
}
