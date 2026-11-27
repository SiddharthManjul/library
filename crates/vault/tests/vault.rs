#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not
//! Envelope encryption and the secrets table.

use library_core::{ErrorCode, TenantId};
use library_testkit::{TestDb, seed_two_tenants};
use library_vault::{
    MasterKeys, MasterKeysError, RotateReport, SecretId, Vault, open, rewrap, seal,
};
use uuid::Uuid;

const K1: &str = "1:0101010101010101010101010101010101010101010101010101010101010101";
const K2: &str = "2:0202020202020202020202020202020202020202020202020202020202020202";
const SECRET: &[u8] = b"xoxp-super-secret-slack-token-1234567890";

fn keys(v: &str) -> MasterKeys {
    MasterKeys::parse(v).unwrap()
}

#[test]
fn seals_and_opens() {
    let (k, t, id) = (keys(K1), TenantId::new(), Uuid::new_v4());
    let s = seal(&k, t, id, SECRET).unwrap();
    assert_eq!(&**open(&k, t, id, &s).unwrap(), SECRET);
    assert!(
        !s.ciphertext.windows(SECRET.len()).any(|w| w == SECRET),
        "ciphertext must not contain the plaintext"
    );
    assert_eq!(s.key_version, 1);
}

#[test]
fn every_seal_is_different_even_for_the_same_secret() {
    let (k, t, id) = (keys(K1), TenantId::new(), Uuid::new_v4());
    let (a, b) = (
        seal(&k, t, id, SECRET).unwrap(),
        seal(&k, t, id, SECRET).unwrap(),
    );
    assert_ne!(a.ciphertext, b.ciphertext);
    assert_ne!(
        a.wrapped_dek, b.wrapped_dek,
        "each secret has its own data key"
    );
}

#[test]
fn a_row_cannot_be_opened_for_another_tenant_or_another_id() {
    let (k, t, id) = (keys(K1), TenantId::new(), Uuid::new_v4());
    let s = seal(&k, t, id, SECRET).unwrap();
    assert!(open(&k, TenantId::new(), id, &s).is_err(), "another tenant");
    assert!(
        open(&k, t, Uuid::new_v4(), &s).is_err(),
        "another secret id"
    );
}

#[test]
fn any_change_to_the_stored_bytes_is_detected() {
    let (k, t, id) = (keys(K1), TenantId::new(), Uuid::new_v4());
    let s = seal(&k, t, id, SECRET).unwrap();
    for i in 0..s.ciphertext.len() {
        let mut bad = s.clone();
        bad.ciphertext[i] ^= 1;
        assert!(open(&k, t, id, &bad).is_err(), "ciphertext byte {i}");
    }
    for i in 0..s.wrapped_dek.len() {
        let mut bad = s.clone();
        bad.wrapped_dek[i] ^= 1;
        assert!(open(&k, t, id, &bad).is_err(), "wrapped key byte {i}");
    }
    let mut truncated = s.clone();
    truncated.ciphertext.truncate(10);
    assert!(open(&k, t, id, &truncated).is_err());
    let mut swapped = s;
    swapped.wrapped_dek = seal(&k, t, id, b"other").unwrap().wrapped_dek;
    assert!(
        open(&k, t, id, &swapped).is_err(),
        "a data key from another secret must not open this one"
    );
}

#[test]
fn the_wrong_master_key_cannot_open_and_failures_reveal_nothing() {
    let (t, id) = (TenantId::new(), Uuid::new_v4());
    let s = seal(&keys(K1), t, id, SECRET).unwrap();
    let other = keys("1:0909090909090909090909090909090909090909090909090909090909090909");
    let e = open(&other, t, id, &s).unwrap_err();
    assert_eq!(e.code, ErrorCode::Internal);
    assert!(!e.message.to_lowercase().contains("tag") && !e.message.contains("key"));
    // A version we do not hold is the same error, not a distinct one an attacker could probe.
    assert_eq!(open(&keys(K2), t, id, &s).unwrap_err(), e);
}

#[test]
fn rotation_rewraps_the_data_key_and_leaves_the_ciphertext_alone() {
    let (t, id) = (TenantId::new(), Uuid::new_v4());
    let old = seal(&keys(K1), t, id, SECRET).unwrap();
    let both = keys(&format!("{K1},{K2}"));
    assert_eq!(
        both.current_version(),
        2,
        "the highest version wraps new keys"
    );
    let rotated = rewrap(&both, t, id, &old).unwrap();
    assert_eq!(rotated.key_version, 2);
    assert_eq!(rotated.ciphertext, old.ciphertext);
    // Once rotated, the old master key can be retired.
    assert_eq!(&**open(&keys(K2), t, id, &rotated).unwrap(), SECRET);
    assert!(open(&keys(K1), t, id, &rotated).is_err());
}

#[test]
fn master_key_parsing_is_strict_and_never_echoes_the_input() {
    assert_eq!(MasterKeys::parse("").unwrap_err(), MasterKeysError::Empty);
    assert_eq!(
        MasterKeys::parse("1:abc").unwrap_err(),
        MasterKeysError::Malformed
    );
    assert_eq!(
        MasterKeys::parse("x:0101010101010101010101010101010101010101010101010101010101010101")
            .unwrap_err(),
        MasterKeysError::Malformed
    );
    assert_eq!(
        MasterKeys::parse("0101010101010101010101010101010101010101010101010101010101010101")
            .unwrap_err(),
        MasterKeysError::Malformed
    );
    assert_eq!(
        MasterKeys::parse(&format!("{K1},{K1}")).unwrap_err(),
        MasterKeysError::Duplicate(1)
    );
    let shown = format!(
        "{:?} {:?}",
        keys(K1),
        MasterKeys::parse("1:zz").unwrap_err()
    );
    assert!(!shown.contains("0101") && !shown.contains("zz"), "{shown}");
}

async fn vault(db: &TestDb, k: &str) -> Vault {
    Vault::new(db.vault.clone(), keys(k))
}

#[tokio::test]
async fn stores_reads_and_replaces_secrets() -> anyhow::Result<()> {
    let db = TestDb::new().await?;
    let fx = seed_two_tenants(&db).await?;
    let (v, t) = (vault(&db, K1).await, TenantId(fx.a.id));
    let id = v.put(t, SECRET).await?;
    assert_eq!(&**v.get(t, id).await?, SECRET);
    v.replace(t, id, b"refreshed-token").await?;
    assert_eq!(&**v.get(t, id).await?, b"refreshed-token");
    assert_eq!(
        v.replace(t, SecretId(Uuid::new_v4()), b"x")
            .await
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    db.drop_db().await
}

#[tokio::test]
async fn the_database_never_holds_the_plaintext_and_tenants_cannot_read_each_other()
-> anyhow::Result<()> {
    let db = TestDb::new().await?;
    let fx = seed_two_tenants(&db).await?;
    let (v, a, b) = (vault(&db, K1).await, TenantId(fx.a.id), TenantId(fx.b.id));
    let id = v.put(a, SECRET).await?;

    let (ct, dek): (Vec<u8>, Vec<u8>) =
        sqlx::query_as("SELECT ciphertext, wrapped_dek FROM secrets WHERE id = $1")
            .bind(id.0)
            .fetch_one(&db.admin)
            .await?;
    for blob in [&ct, &dek] {
        assert!(
            !blob.windows(8).any(|w| SECRET.windows(8).any(|s| s == w)),
            "no plaintext fragment in the table"
        );
    }
    // Tenant B cannot read tenant A's secret by id, through the vault or through the role directly.
    assert_eq!(v.get(b, id).await.unwrap_err().code, ErrorCode::NotFound);
    let direct: i64 = {
        let mut tx = db.vault.begin().await?;
        sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
            .bind(fx.b.id.to_string())
            .execute(&mut *tx)
            .await?;
        sqlx::query_scalar("SELECT count(*) FROM secrets WHERE id = $1")
            .bind(id.0)
            .fetch_one(&mut *tx)
            .await?
    };
    assert_eq!(direct, 0);
    // And the runtime role cannot read the table at all.
    assert!(
        sqlx::query("SELECT 1 FROM secrets")
            .fetch_all(&db.app)
            .await
            .is_err()
    );
    db.drop_db().await
}

#[tokio::test]
async fn a_row_copied_to_another_tenant_does_not_open() -> anyhow::Result<()> {
    let db = TestDb::new().await?;
    let fx = seed_two_tenants(&db).await?;
    let (v, a, b) = (vault(&db, K1).await, TenantId(fx.a.id), TenantId(fx.b.id));
    let id = v.put(a, SECRET).await?;
    // An attacker with write access to the table moves the row to another tenant.
    sqlx::query("UPDATE secrets SET tenant_id = $2 WHERE id = $1")
        .bind(id.0)
        .bind(b.0)
        .execute(&db.admin)
        .await?;
    assert!(
        v.get(b, id).await.is_err(),
        "the tenant is bound into the encryption"
    );
    db.drop_db().await
}

#[tokio::test]
async fn rotating_keys_rewraps_every_secret_without_changing_what_they_say() -> anyhow::Result<()> {
    let db = TestDb::new().await?;
    let fx = seed_two_tenants(&db).await?;
    let t = TenantId(fx.a.id);
    let old = vault(&db, K1).await;
    let (x, y) = (old.put(t, b"first").await?, old.put(t, b"second").await?);
    let new = vault(&db, &format!("{K1},{K2}")).await;
    // The fixture's own secrets row is random bytes no vault sealed, so it cannot be unwrapped. It must be
    // reported, and it must not stop the two real secrets from rotating.
    assert_eq!(
        new.rotate(t).await?,
        RotateReport {
            rewrapped: 2,
            failed: 1
        }
    );
    assert_eq!(
        new.rotate(t).await?,
        RotateReport {
            rewrapped: 0,
            failed: 1
        },
        "a second pass has nothing left but the broken row"
    );
    let only_new = vault(&db, K2).await;
    assert_eq!(&**only_new.get(t, x).await?, b"first");
    assert_eq!(&**only_new.get(t, y).await?, b"second");
    assert!(
        old.get(t, x).await.is_err(),
        "the retired key no longer works"
    );
    db.drop_db().await
}

#[tokio::test]
async fn the_vault_prints_no_key_material() {
    let db = TestDb::new().await.unwrap();
    let v = vault(&db, K1).await;
    let shown = format!("{v:?}");
    assert!(!shown.contains("0101"), "{shown}");
    db.drop_db().await.unwrap();
}
