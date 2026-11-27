use std::collections::BTreeMap;

use zeroize::Zeroizing;

pub const KEY_LEN: usize = 32;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum MasterKeysError {
    #[error("no master key is configured")]
    Empty,
    #[error("master key entries look like `<version>:<64 hex characters>`")]
    Malformed,
    #[error("master key version {0} appears twice")]
    Duplicate(u32),
}

/// Versioned master keys held outside the database (an environment value in v0, a cloud key service
/// later). The highest version wraps new data keys; older versions stay only to unwrap until every
/// row has been rotated.
pub struct MasterKeys {
    keys: BTreeMap<u32, Zeroizing<[u8; KEY_LEN]>>,
}

impl MasterKeys {
    /// Parses `1:<hex>,2:<hex>`. The error never repeats the input, which holds key material.
    pub fn parse(value: &str) -> Result<Self, MasterKeysError> {
        let mut keys = BTreeMap::new();
        for entry in value.split(',').map(str::trim).filter(|e| !e.is_empty()) {
            let (version, hexkey) = entry.split_once(':').ok_or(MasterKeysError::Malformed)?;
            let version: u32 = version
                .trim()
                .parse()
                .map_err(|_| MasterKeysError::Malformed)?;
            let raw =
                Zeroizing::new(hex::decode(hexkey.trim()).map_err(|_| MasterKeysError::Malformed)?);
            let mut key = Zeroizing::new([0u8; KEY_LEN]);
            if raw.len() != KEY_LEN {
                return Err(MasterKeysError::Malformed);
            }
            key.copy_from_slice(&raw);
            if keys.insert(version, key).is_some() {
                return Err(MasterKeysError::Duplicate(version));
            }
        }
        if keys.is_empty() {
            Err(MasterKeysError::Empty)
        } else {
            Ok(Self { keys })
        }
    }

    /// From the `LIBRARY_MASTER_KEYS` environment variable.
    pub fn from_env() -> Result<Self, MasterKeysError> {
        Self::parse(&std::env::var("LIBRARY_MASTER_KEYS").unwrap_or_default())
    }

    pub fn current_version(&self) -> u32 {
        self.keys.keys().next_back().copied().unwrap_or(0)
    }

    pub(crate) fn get(&self, version: u32) -> Option<&[u8; KEY_LEN]> {
        self.keys.get(&version).map(|k| &**k)
    }
}

/// Never prints key material.
impl std::fmt::Debug for MasterKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MasterKeys(versions: {:?})",
            self.keys.keys().collect::<Vec<_>>()
        )
    }
}
