//! The member's session on this device, kept in the OS keychain: the tenant and user it is for, and the
//! current token pair. The desktop app never sees any of it.

use library_core::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::keychain::{KeychainError, SecretStore};

pub const KEYCHAIN_ITEM: &str = "session-v1";

#[derive(Clone, Serialize, Deserialize)]
pub struct StoredSession {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub access: String,
    pub refresh: String,
    /// Unix seconds.
    pub access_expires_at: i64,
}

/// Never prints a token.
impl std::fmt::Debug for StoredSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredSession")
            .field("tenant_id", &self.tenant_id)
            .field("user_id", &self.user_id)
            .field("access_expires_at", &self.access_expires_at)
            .finish_non_exhaustive()
    }
}

impl StoredSession {
    pub fn save(&self, store: &dyn SecretStore) -> Result<(), KeychainError> {
        let bytes = Zeroizing::new(serde_json::to_vec(self).map_err(|_| KeychainError::Malformed)?);
        store.set(KEYCHAIN_ITEM, &bytes)
    }

    /// `Ok(None)` when nobody is signed in. A damaged item is an error, never a silent sign-out.
    pub fn load(store: &dyn SecretStore) -> Result<Option<Self>, KeychainError> {
        match store.get(KEYCHAIN_ITEM)? {
            None => Ok(None),
            Some(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|_| KeychainError::Malformed),
        }
    }

    pub fn clear(store: &dyn SecretStore) -> Result<(), KeychainError> {
        store.delete(KEYCHAIN_ITEM)
    }
}
