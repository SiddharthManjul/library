//! Envelope encryption of secrets: plugin tokens, customer-owned app secrets, Slack signing secrets
//! and model API keys. Only this crate (and the dispatcher and model proxy it serves) ever sees a
//! decrypted credential. Secrets never enter logs, traces, errors, a model's context or a replica.

mod envelope;
mod keys;
mod store;

pub use envelope::{Sealed, open, rewrap, seal};
pub use keys::{MasterKeys, MasterKeysError};
pub use store::{RotateReport, SecretId, Vault};
