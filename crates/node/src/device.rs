//! Device keys: an X25519 key for receiving sealed data and an Ed25519 key for signing.
//! The private halves live only in the OS keychain. They are loaded into memory to use and zeroised
//! on drop; there is deliberately no method that returns them.

use ed25519_dalek::{Signer, SigningKey};
use library_core::{DeviceRegistration, TenantId, UserId, device_registration_message};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

use crate::keychain::{KeychainError, SecretStore};

/// Keychain item holding both private keys (32 bytes X25519 followed by 32 bytes Ed25519 seed).
pub const KEYCHAIN_ITEM: &str = "device-keys-v1";

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error(transparent)]
    Keychain(#[from] KeychainError),
    #[error("no secure random source")]
    Random,
}

pub struct DeviceKeys {
    sealing: StaticSecret,
    signing: SigningKey,
}

/// Never prints key material.
impl std::fmt::Debug for DeviceKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeviceKeys(..)")
    }
}

impl DeviceKeys {
    /// Loads the device's keys from the keychain, creating and storing them on first run.
    pub fn load_or_create(store: &dyn SecretStore) -> Result<Self, DeviceError> {
        if let Some(raw) = store.get(KEYCHAIN_ITEM)? {
            let bytes: &[u8; 64] = raw
                .as_slice()
                .try_into()
                .map_err(|_| KeychainError::Malformed)?;
            return Ok(Self::from_bytes(bytes));
        }
        let mut raw = Zeroizing::new([0u8; 64]);
        getrandom::fill(raw.as_mut_slice()).map_err(|_| DeviceError::Random)?;
        let keys = Self::from_bytes(&raw);
        store.set(KEYCHAIN_ITEM, raw.as_slice())?;
        Ok(keys)
    }

    fn from_bytes(b: &[u8; 64]) -> Self {
        let mut x = Zeroizing::new([0u8; 32]);
        let mut s = Zeroizing::new([0u8; 32]);
        x.copy_from_slice(&b[..32]);
        s.copy_from_slice(&b[32..]);
        Self {
            sealing: StaticSecret::from(*x),
            signing: SigningKey::from_bytes(&s),
        }
    }

    /// Removes the keys from the keychain (sign-out on this device, or after revocation).
    pub fn forget(store: &dyn SecretStore) -> Result<(), DeviceError> {
        Ok(store.delete(KEYCHAIN_ITEM)?)
    }

    pub fn sealing_public(&self) -> [u8; 32] {
        PublicKey::from(&self.sealing).to_bytes()
    }

    pub fn signing_public(&self) -> [u8; 32] {
        self.signing.verifying_key().to_bytes()
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing.sign(message).to_bytes()
    }

    /// The payload that registers this device for one tenant and user.
    pub fn registration(&self, tenant: TenantId, user: UserId) -> DeviceRegistration {
        let x25519_public = self.sealing_public();
        DeviceRegistration {
            x25519_public,
            ed25519_public: self.signing_public(),
            proof: self.sign(&device_registration_message(tenant, user, &x25519_public)),
        }
    }
}
