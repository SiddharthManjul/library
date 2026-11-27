//! Insert-only, hash-chained audit log, one chain per tenant.
//!
//! Every row stores `hash = SHA-256(prev_hash || canonical(row))`, so editing or deleting a row in the
//! middle breaks the chain from that point. Truncating the end of the log leaves a valid chain, so the
//! tip is also signed once a day and the signature is kept outside the database (`HeadStore`); the
//! verifier compares the chain with the latest signed head.

mod canonical;
mod head;
mod log;
mod verify;

pub use canonical::{AuditEntry, Row, genesis_hash, hash_payload};
pub use head::{FileHeadStore, HeadError, HeadSigner, HeadStore, SignedHead};
pub use log::AuditLog;
pub use verify::{Problem, Report, verify_chain};
