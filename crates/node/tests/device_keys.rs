//! Device key tests (P0-11): creation, persistence and no leakage. No database needed, so they run on any machine.
#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not

use library_core::{TenantId, UserId};
use library_node::{
    device::{DeviceKeys, KEYCHAIN_ITEM},
    keychain::{MemoryStore, OsKeychain, SecretStore},
};

#[test]
fn keys_are_created_once_and_reloaded_identically() {
    let store = MemoryStore::new();
    let first = DeviceKeys::load_or_create(&store).unwrap();
    let again = DeviceKeys::load_or_create(&store).unwrap();
    assert_eq!(first.sealing_public(), again.sealing_public());
    assert_eq!(first.signing_public(), again.signing_public());
    assert_eq!(store.dump().len(), 1, "one keychain item holds both keys");
    let other = DeviceKeys::load_or_create(&MemoryStore::new()).unwrap();
    assert_ne!(first.sealing_public(), other.sealing_public());
    assert_ne!(first.signing_public(), other.signing_public());
}

#[test]
fn sealing_and_signing_keys_are_independent() {
    let k = DeviceKeys::load_or_create(&MemoryStore::new()).unwrap();
    assert_ne!(k.sealing_public(), k.signing_public());
}

#[test]
fn private_keys_never_appear_in_debug_output_or_the_registration() {
    let store = MemoryStore::new();
    let keys = DeviceKeys::load_or_create(&store).unwrap();
    let private = store.get(KEYCHAIN_ITEM).unwrap().unwrap();
    let reg = keys.registration(TenantId::new(), UserId::new());
    let shown = format!("{keys:?} {reg:?}");
    for half in [&private[..32], &private[32..]] {
        assert!(!shown.contains(&format!("{half:?}")));
        let hex: String = half.iter().map(|b| format!("{b:02x}")).collect();
        assert!(!shown.contains(&hex));
        assert!(!reg.x25519_public.windows(32).any(|w| w == half));
        assert!(!reg.ed25519_public.windows(32).any(|w| w == half));
        assert!(!reg.proof.windows(32).any(|w| w == half));
    }
}

#[test]
fn a_damaged_keychain_item_is_an_error_not_new_keys() {
    let store = MemoryStore::new();
    store.set(KEYCHAIN_ITEM, b"short").unwrap();
    assert!(
        DeviceKeys::load_or_create(&store).is_err(),
        "must not silently replace keys"
    );
}

#[test]
fn forgetting_removes_the_keys() {
    let store = MemoryStore::new();
    let before = DeviceKeys::load_or_create(&store).unwrap().sealing_public();
    DeviceKeys::forget(&store).unwrap();
    assert!(store.get(KEYCHAIN_ITEM).unwrap().is_none());
    assert_ne!(
        DeviceKeys::load_or_create(&store).unwrap().sealing_public(),
        before
    );
}

/// Needs a real OS keychain: Windows Credential Manager, or a running Secret Service on Linux.
/// Run with `cargo nextest run -p library-node --run-ignored only`.
#[test]
#[ignore = "needs an OS keychain"]
fn the_os_keychain_stores_and_returns_the_keys() {
    let store = OsKeychain::new("library-ide-test");
    let _ = DeviceKeys::forget(&store);
    let first = DeviceKeys::load_or_create(&store).unwrap();
    let again = DeviceKeys::load_or_create(&store).unwrap();
    assert_eq!(first.sealing_public(), again.sealing_public());
    DeviceKeys::forget(&store).unwrap();
}
