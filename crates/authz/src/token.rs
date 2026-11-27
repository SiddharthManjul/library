//! Short-lived access tokens signed with Ed25519.
//!
//! Format: `v1.<claims>.<signature>`, both base64url without padding. The signature covers a fixed
//! domain tag followed by the claims bytes, so a signature made for another purpose never verifies.
//! Tokens are internal (node to gateway), so the format is ours and kept deliberately small.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use library_core::{DeviceId, ErrorCode, LibraryError, TenantId, UserId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Access tokens live 15 minutes.
pub const ACCESS_TTL_SECS: i64 = 15 * 60;
const DOMAIN: &[u8] = b"library-access-v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: UserId,
    pub tid: TenantId,
    pub did: DeviceId,
    pub iat: i64,
    pub exp: i64,
    pub jti: Uuid,
}

fn unauthenticated() -> LibraryError {
    LibraryError::new(ErrorCode::Unauthenticated, "Sign in again to continue.")
}

/// Holds the server's private signing key. Never logged or serialised.
pub struct TokenSigner {
    key: SigningKey,
}

impl TokenSigner {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(&seed),
        }
    }

    pub fn generate() -> Result<Self, LibraryError> {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).map_err(|_| LibraryError::internal())?;
        Ok(Self::from_seed(seed))
    }

    pub fn verifier(&self) -> TokenVerifier {
        TokenVerifier {
            key: self.key.verifying_key(),
        }
    }

    pub fn issue(
        &self,
        user: UserId,
        tenant: TenantId,
        device: DeviceId,
        now: OffsetDateTime,
    ) -> Result<(String, AccessClaims), LibraryError> {
        let claims = AccessClaims {
            sub: user,
            tid: tenant,
            did: device,
            iat: now.unix_timestamp(),
            exp: now.unix_timestamp() + ACCESS_TTL_SECS,
            jti: Uuid::new_v4(),
        };
        let body = serde_json::to_vec(&claims).map_err(|_| LibraryError::internal())?;
        let sig = self.key.sign(&[DOMAIN, &body].concat());
        Ok((
            format!("v1.{}.{}", B64.encode(&body), B64.encode(sig.to_bytes())),
            claims,
        ))
    }
}

impl std::fmt::Debug for TokenSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TokenSigner(..)")
    }
}

#[derive(Clone)]
pub struct TokenVerifier {
    key: VerifyingKey,
}

impl TokenVerifier {
    pub fn from_public_bytes(bytes: &[u8; 32]) -> Result<Self, LibraryError> {
        VerifyingKey::from_bytes(bytes)
            .map(|key| Self { key })
            .map_err(|_| LibraryError::internal())
    }

    /// Checks signature first, then expiry. Any malformed input is `Unauthenticated`.
    pub fn verify(&self, token: &str, now: OffsetDateTime) -> Result<AccessClaims, LibraryError> {
        let mut parts = token.split('.');
        let (Some("v1"), Some(body), Some(sig), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(unauthenticated());
        };
        let body = B64.decode(body).map_err(|_| unauthenticated())?;
        let sig: [u8; 64] = B64
            .decode(sig)
            .map_err(|_| unauthenticated())?
            .try_into()
            .map_err(|_| unauthenticated())?;
        self.key
            .verify(&[DOMAIN, &body].concat(), &Signature::from_bytes(&sig))
            .map_err(|_| unauthenticated())?;
        let claims: AccessClaims = serde_json::from_slice(&body).map_err(|_| unauthenticated())?;
        if now.unix_timestamp() >= claims.exp {
            return Err(LibraryError::new(
                ErrorCode::TokenExpired,
                "Your session expired. Refreshing it now.",
            ));
        }
        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (UserId, TenantId, DeviceId) {
        (UserId::new(), TenantId::new(), DeviceId::new())
    }

    #[test]
    fn roundtrip_and_expiry() {
        let s = TokenSigner::generate().unwrap();
        let (u, t, d) = ids();
        let now = OffsetDateTime::now_utc();
        let (tok, claims) = s.issue(u, t, d, now).unwrap();
        assert_eq!(s.verifier().verify(&tok, now).unwrap(), claims);
        let late = now + time::Duration::seconds(ACCESS_TTL_SECS);
        assert_eq!(
            s.verifier().verify(&tok, late).unwrap_err().code,
            ErrorCode::TokenExpired
        );
    }

    #[test]
    fn tampering_is_rejected() {
        let s = TokenSigner::generate().unwrap();
        let (u, t, d) = ids();
        let now = OffsetDateTime::now_utc();
        let (tok, _) = s.issue(u, t, d, now).unwrap();
        let parts: Vec<&str> = tok.split('.').collect();
        // Swap in claims for another tenant, keep the old signature.
        let (other, _) = s.issue(u, TenantId::new(), d, now).unwrap();
        let forged = format!("v1.{}.{}", other.split('.').nth(1).unwrap(), parts[2]);
        assert_eq!(
            s.verifier().verify(&forged, now).unwrap_err().code,
            ErrorCode::Unauthenticated
        );
        // Flip a signature byte.
        let mut sig = B64.decode(parts[2]).unwrap();
        sig[0] ^= 1;
        let bad = format!("v1.{}.{}", parts[1], B64.encode(sig));
        assert_eq!(
            s.verifier().verify(&bad, now).unwrap_err().code,
            ErrorCode::Unauthenticated
        );
    }

    #[test]
    fn another_servers_key_is_rejected() {
        let (a, b) = (
            TokenSigner::generate().unwrap(),
            TokenSigner::generate().unwrap(),
        );
        let (u, t, d) = ids();
        let now = OffsetDateTime::now_utc();
        let (tok, _) = a.issue(u, t, d, now).unwrap();
        assert!(b.verifier().verify(&tok, now).is_err());
    }

    #[test]
    fn garbage_is_rejected_without_panic() {
        let v = TokenSigner::generate().unwrap().verifier();
        for junk in [
            "",
            "v1",
            "v1..",
            "v1.a.b",
            "v2.a.b",
            "a.b.c.d",
            "v1.!!!.!!!",
        ] {
            assert_eq!(
                v.verify(junk, OffsetDateTime::now_utc()).unwrap_err().code,
                ErrorCode::Unauthenticated,
                "{junk}"
            );
        }
    }
}
