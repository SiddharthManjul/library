use serde::{Deserialize, Serialize};

use crate::{TenantId, UserId};

/// Domain tag for the device registration proof. A signature made for any other purpose never verifies.
pub const DEVICE_PROOF_DOMAIN: &[u8] = b"library-device-register-v1\0";

/// What a node sends to register a device: both public keys and a proof it holds the signing key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRegistration {
    pub x25519_public: [u8; 32],
    pub ed25519_public: [u8; 32],
    /// Ed25519 signature over `device_registration_message(tenant, user, x25519_public)`.
    #[serde(with = "sig_bytes")]
    pub proof: [u8; 64],
}

mod sig_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        v.as_slice().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v = Vec::<u8>::deserialize(d)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom("proof must be 64 bytes"))
    }
}

/// The exact bytes a device signs with its Ed25519 key to register: they bind the signature to one
/// tenant, one user and one X25519 sealing key, so a proof cannot be replayed elsewhere.
pub fn device_registration_message(
    tenant: TenantId,
    user: UserId,
    x25519_public: &[u8; 32],
) -> Vec<u8> {
    let mut m = Vec::with_capacity(DEVICE_PROOF_DOMAIN.len() + 16 + 16 + 32);
    m.extend_from_slice(DEVICE_PROOF_DOMAIN);
    m.extend_from_slice(tenant.0.as_bytes());
    m.extend_from_slice(user.0.as_bytes());
    m.extend_from_slice(x25519_public);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_binds_every_field() {
        let (t, u, k) = (TenantId::new(), UserId::new(), [7u8; 32]);
        let base = device_registration_message(t, u, &k);
        assert_ne!(base, device_registration_message(TenantId::new(), u, &k));
        assert_ne!(base, device_registration_message(t, UserId::new(), &k));
        assert_ne!(base, device_registration_message(t, u, &[8u8; 32]));
        assert!(base.starts_with(DEVICE_PROOF_DOMAIN));
    }
}
