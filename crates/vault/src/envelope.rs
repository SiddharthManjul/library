//! The envelope: `plaintext --AES-256-GCM(dek)--> ciphertext`, `dek --AES-256-GCM(master)--> wrapped_dek`.
//! Both layers bind the tenant and the secret id as associated data, and the wrap also binds the master
//! key version, so a row moved to another tenant, another id or another version fails to open.

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use library_core::{LibraryError, TenantId};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::keys::{KEY_LEN, MasterKeys};

const NONCE_LEN: usize = 12;
const PAYLOAD_TAG: &[u8] = b"library-secret-v1\0";
const WRAP_TAG: &[u8] = b"library-dek-v1\0";

/// What is stored in the `secrets` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealed {
    pub ciphertext: Vec<u8>,
    pub wrapped_dek: Vec<u8>,
    pub key_version: u32,
}

fn fail() -> LibraryError {
    // One message for every failure: callers learn nothing about which check failed.
    LibraryError::internal()
}

fn payload_aad(tenant: TenantId, id: Uuid) -> Vec<u8> {
    [PAYLOAD_TAG, tenant.0.as_bytes(), id.as_bytes()].concat()
}

fn wrap_aad(tenant: TenantId, id: Uuid, version: u32) -> Vec<u8> {
    [
        WRAP_TAG,
        tenant.0.as_bytes(),
        id.as_bytes(),
        &version.to_be_bytes(),
    ]
    .concat()
}

fn encrypt(key: &[u8; KEY_LEN], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, LibraryError> {
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce).map_err(|_| fail())?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| fail())?;
    let ct = cipher
        .encrypt(
            &Nonce::try_from(nonce.as_slice()).map_err(|_| fail())?,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| fail())?;
    Ok([nonce.as_slice(), &ct].concat())
}

fn decrypt(
    key: &[u8; KEY_LEN],
    aad: &[u8],
    blob: &[u8],
) -> Result<Zeroizing<Vec<u8>>, LibraryError> {
    if blob.len() < NONCE_LEN + 16 {
        return Err(fail());
    }
    let (nonce, ct) = blob.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| fail())?;
    cipher
        .decrypt(
            &Nonce::try_from(nonce).map_err(|_| fail())?,
            Payload { msg: ct, aad },
        )
        .map(Zeroizing::new)
        .map_err(|_| fail())
}

/// Encrypts `plaintext` under a fresh data key and wraps that key with the current master key.
pub fn seal(
    keys: &MasterKeys,
    tenant: TenantId,
    id: Uuid,
    plaintext: &[u8],
) -> Result<Sealed, LibraryError> {
    let version = keys.current_version();
    let master = keys.get(version).ok_or_else(fail)?;
    let mut dek = Zeroizing::new([0u8; KEY_LEN]);
    getrandom::fill(dek.as_mut_slice()).map_err(|_| fail())?;
    Ok(Sealed {
        ciphertext: encrypt(&dek, &payload_aad(tenant, id), plaintext)?,
        wrapped_dek: encrypt(master, &wrap_aad(tenant, id, version), dek.as_slice())?,
        key_version: version,
    })
}

fn unwrap_dek(
    keys: &MasterKeys,
    tenant: TenantId,
    id: Uuid,
    s: &Sealed,
) -> Result<Zeroizing<[u8; KEY_LEN]>, LibraryError> {
    let master = keys.get(s.key_version).ok_or_else(fail)?;
    let raw = decrypt(master, &wrap_aad(tenant, id, s.key_version), &s.wrapped_dek)?;
    let mut dek = Zeroizing::new([0u8; KEY_LEN]);
    if raw.len() != KEY_LEN {
        return Err(fail());
    }
    dek.copy_from_slice(&raw);
    Ok(dek)
}

/// Opens a secret. The result is zeroised when dropped.
pub fn open(
    keys: &MasterKeys,
    tenant: TenantId,
    id: Uuid,
    s: &Sealed,
) -> Result<Zeroizing<Vec<u8>>, LibraryError> {
    let dek = unwrap_dek(keys, tenant, id, s)?;
    decrypt(&dek, &payload_aad(tenant, id), &s.ciphertext)
}

/// Re-wraps the data key under the current master key. The ciphertext is untouched.
pub fn rewrap(
    keys: &MasterKeys,
    tenant: TenantId,
    id: Uuid,
    s: &Sealed,
) -> Result<Sealed, LibraryError> {
    let dek = unwrap_dek(keys, tenant, id, s)?;
    let version = keys.current_version();
    let master = keys.get(version).ok_or_else(fail)?;
    Ok(Sealed {
        ciphertext: s.ciphertext.clone(),
        wrapped_dek: encrypt(master, &wrap_aad(tenant, id, version), dek.as_slice())?,
        key_version: version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (MasterKeys, TenantId, Uuid, Sealed) {
        let keys =
            MasterKeys::parse("1:0101010101010101010101010101010101010101010101010101010101010101")
                .unwrap();
        let (t, id) = (TenantId::new(), Uuid::new_v4());
        let s = seal(&keys, t, id, b"secret").unwrap();
        (keys, t, id, s)
    }

    /// Each layer must refuse another tenant or id on its own, so one regression cannot hide behind the other.
    #[test]
    fn the_payload_layer_binds_tenant_and_id() {
        let (keys, t, id, s) = fixture();
        let dek = unwrap_dek(&keys, t, id, &s).unwrap();
        assert!(decrypt(&dek, &payload_aad(t, id), &s.ciphertext).is_ok());
        assert!(decrypt(&dek, &payload_aad(TenantId::new(), id), &s.ciphertext).is_err());
        assert!(decrypt(&dek, &payload_aad(t, Uuid::new_v4()), &s.ciphertext).is_err());
    }

    #[test]
    fn the_key_wrap_layer_binds_tenant_id_and_version() {
        let (keys, t, id, s) = fixture();
        assert!(unwrap_dek(&keys, t, id, &s).is_ok());
        assert!(unwrap_dek(&keys, TenantId::new(), id, &s).is_err());
        assert!(unwrap_dek(&keys, t, Uuid::new_v4(), &s).is_err());
        // Claiming a different master key version must fail even when that version holds the same key bytes.
        let same_bytes = MasterKeys::parse("1:0101010101010101010101010101010101010101010101010101010101010101,2:0101010101010101010101010101010101010101010101010101010101010101").unwrap();
        let relabelled = Sealed {
            key_version: 2,
            ..s
        };
        assert!(unwrap_dek(&same_bytes, t, id, &relabelled).is_err());
    }
}
