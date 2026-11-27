//! Roles, scopes, folds and entitlements; sessions and tokens; sets the Postgres
//! session variables used by row-level security.

pub mod context;
pub mod devices;
pub mod scoped;
pub mod session;
pub mod token;

pub use context::build_ctx;
pub use scoped::begin;
pub use session::{SessionService, TokenPair};
pub use token::{AccessClaims, TokenSigner, TokenVerifier};
