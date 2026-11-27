//! Node and server together: a node registers its device keys with a real server database.
#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not

use library_authz::devices::register;
use library_core::{TenantId, UserId};
use library_node::{device::DeviceKeys, keychain::MemoryStore};
use library_testkit::{TestDb, seed_two_tenants};
use time::OffsetDateTime;

#[tokio::test]
async fn a_node_registers_with_the_server() -> anyhow::Result<()> {
    let db = TestDb::new().await?;
    let fx = seed_two_tenants(&db).await?;
    let keys = DeviceKeys::load_or_create(&MemoryStore::new())?;
    let (t, u) = (TenantId(fx.a.id), UserId(fx.a.user));
    let id = register(
        &db.auth,
        t,
        u,
        &keys.registration(t, u),
        OffsetDateTime::now_utc(),
    )
    .await?;
    let stored: (Vec<u8>, Vec<u8>) =
        sqlx::query_as("SELECT public_key, signing_public_key FROM devices WHERE id = $1")
            .bind(id.0)
            .fetch_one(&db.admin)
            .await?;
    assert_eq!(stored.0, keys.sealing_public().to_vec());
    assert_eq!(stored.1, keys.signing_public().to_vec());
    // The same keys cannot be registered for another user's account.
    let other = UserId(fx.b.user);
    assert!(
        register(
            &db.auth,
            t,
            other,
            &keys.registration(t, other),
            OffsetDateTime::now_utc()
        )
        .await
        .is_err()
    );
    db.drop_db().await
}
