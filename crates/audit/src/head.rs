//! The signed chain tip. Once a day the server signs `(tenant, date, seq, hash)` and stores the
//! signature somewhere the database's writers cannot reach (a file today, R2 later).

use std::{fs, io::Write, path::PathBuf};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use library_core::TenantId;
use serde::{Deserialize, Serialize};

const HEAD_TAG: &[u8] = b"library-audit-head-v1\0";

#[derive(Debug, thiserror::Error)]
pub enum HeadError {
    #[error("the head store could not be read or written")]
    Store,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedHead {
    pub tenant: TenantId,
    /// Calendar day in UTC, `YYYY-MM-DD`.
    pub day: String,
    pub seq: i64,
    #[serde(with = "hex_32")]
    pub hash: [u8; 32],
    #[serde(with = "hex_64")]
    pub signature: [u8; 64],
}

mod hex_32 {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(v))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let v = hex::decode(String::deserialize(d)?).map_err(serde::de::Error::custom)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom("expected 32 bytes"))
    }
}
mod hex_64 {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(v))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v = hex::decode(String::deserialize(d)?).map_err(serde::de::Error::custom)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom("expected 64 bytes"))
    }
}

fn message(tenant: TenantId, day: &str, seq: i64, hash: &[u8; 32]) -> Vec<u8> {
    [
        HEAD_TAG,
        tenant.0.as_bytes(),
        day.as_bytes(),
        b"\0",
        &seq.to_be_bytes(),
        hash,
    ]
    .concat()
}

pub struct HeadSigner {
    key: SigningKey,
}

impl HeadSigner {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(&seed),
        }
    }

    pub fn public_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }

    pub fn sign(&self, tenant: TenantId, day: &str, seq: i64, hash: [u8; 32]) -> SignedHead {
        let signature = self.key.sign(&message(tenant, day, seq, &hash)).to_bytes();
        SignedHead {
            tenant,
            day: day.to_string(),
            seq,
            hash,
            signature,
        }
    }
}

impl SignedHead {
    pub fn is_valid(&self, key: &VerifyingKey) -> bool {
        key.verify(
            &message(self.tenant, &self.day, self.seq, &self.hash),
            &Signature::from_bytes(&self.signature),
        )
        .is_ok()
    }
}

/// Where signed heads live. Implementations must be append-only from the database's point of view.
pub trait HeadStore {
    fn append(&self, head: &SignedHead) -> Result<(), HeadError>;
    fn heads(&self, tenant: TenantId) -> Result<Vec<SignedHead>, HeadError>;
}

/// One JSON object per line in a file outside the database.
pub struct FileHeadStore {
    path: PathBuf,
}

impl FileHeadStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl HeadStore for FileHeadStore {
    fn append(&self, head: &SignedHead) -> Result<(), HeadError> {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|_| HeadError::Store)?;
        let line = serde_json::to_string(head).map_err(|_| HeadError::Store)?;
        writeln!(f, "{line}").map_err(|_| HeadError::Store)
    }

    fn heads(&self, tenant: TenantId) -> Result<Vec<SignedHead>, HeadError> {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(_) => return Err(HeadError::Store),
        };
        // A line that does not parse is skipped here and shows up as a missing head, never as a trusted one.
        Ok(text
            .lines()
            .filter_map(|l| serde_json::from_str::<SignedHead>(l).ok())
            .filter(|h| h.tenant == tenant)
            .collect())
    }
}
