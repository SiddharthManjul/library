//! Secret storage behind a small trait. Production uses the OS keychain (Windows Credential Manager,
//! Linux Secret Service); tests use memory. Secrets are handed back in `Zeroizing` buffers.

use std::{collections::HashMap, sync::Mutex};

use zeroize::Zeroizing;

#[derive(Debug, thiserror::Error)]
pub enum KeychainError {
    #[error("the OS keychain is unavailable: {0}")]
    Unavailable(String),
    #[error("stored secret is malformed")]
    Malformed,
}

pub trait SecretStore: Send + Sync {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, KeychainError>;
    fn set(&self, name: &str, secret: &[u8]) -> Result<(), KeychainError>;
    fn delete(&self, name: &str) -> Result<(), KeychainError>;
}

/// The operating system's keychain.
pub struct OsKeychain {
    service: String,
}

impl OsKeychain {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, name: &str) -> Result<keyring::Entry, KeychainError> {
        keyring::Entry::new(&self.service, name)
            .map_err(|e| KeychainError::Unavailable(e.to_string()))
    }
}

impl SecretStore for OsKeychain {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, KeychainError> {
        match self.entry(name)?.get_secret() {
            Ok(v) => Ok(Some(Zeroizing::new(v))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(KeychainError::Unavailable(e.to_string())),
        }
    }

    fn set(&self, name: &str, secret: &[u8]) -> Result<(), KeychainError> {
        self.entry(name)?
            .set_secret(secret)
            .map_err(|e| KeychainError::Unavailable(e.to_string()))
    }

    fn delete(&self, name: &str) -> Result<(), KeychainError> {
        match self.entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(KeychainError::Unavailable(e.to_string())),
        }
    }
}

/// In-memory store for tests.
#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, Vec<u8>>>);

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything stored, for tests that check what was written.
    pub fn dump(&self) -> Vec<(String, Vec<u8>)> {
        self.0
            .lock()
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, KeychainError> {
        let m = self.0.lock().map_err(|_| KeychainError::Malformed)?;
        Ok(m.get(name).map(|v| Zeroizing::new(v.clone())))
    }

    fn set(&self, name: &str, secret: &[u8]) -> Result<(), KeychainError> {
        self.0
            .lock()
            .map_err(|_| KeychainError::Malformed)?
            .insert(name.to_string(), secret.to_vec());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), KeychainError> {
        self.0
            .lock()
            .map_err(|_| KeychainError::Malformed)?
            .remove(name);
        Ok(())
    }
}

/// Development and test only: secrets in a plain JSON file. It exists so end-to-end tests can seed a session
/// without a desktop keychain, and it is compiled out of release builds, so an installed app can never be
/// pointed at a plaintext file by an environment variable.
#[cfg(debug_assertions)]
pub struct DevFileStore {
    path: std::path::PathBuf,
}

#[cfg(debug_assertions)]
impl DevFileStore {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn read(&self) -> std::collections::HashMap<String, String> {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn write(&self, m: &std::collections::HashMap<String, String>) -> Result<(), KeychainError> {
        let text = serde_json::to_string(m).map_err(|_| KeychainError::Malformed)?;
        std::fs::write(&self.path, text).map_err(|e| KeychainError::Unavailable(e.to_string()))
    }
}

#[cfg(debug_assertions)]
impl SecretStore for DevFileStore {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, KeychainError> {
        Ok(self
            .read()
            .get(name)
            .map(|v| Zeroizing::new(v.clone().into_bytes())))
    }

    fn set(&self, name: &str, secret: &[u8]) -> Result<(), KeychainError> {
        let mut m = self.read();
        m.insert(
            name.to_string(),
            String::from_utf8(secret.to_vec()).map_err(|_| KeychainError::Malformed)?,
        );
        self.write(&m)
    }

    fn delete(&self, name: &str) -> Result<(), KeychainError> {
        let mut m = self.read();
        m.remove(name);
        self.write(&m)
    }
}
