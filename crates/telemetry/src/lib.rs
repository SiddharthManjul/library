//! Tracing, log redaction and the decision log.
//!
//! Secrets are scrubbed at the output boundary: every formatted log line passes through `scrub` before
//! it is written, so a token that reaches a log through a message, a `Debug` dump, an error string or a
//! span field is caught the same way, whichever crate logged it.

mod decision;
mod redact;
mod tracing_setup;

pub use decision::{DecisionKind, DecisionLog};
pub use redact::{WITHHELD, patterns_loaded, scrub, scrub_json, strip_ansi};
pub use tracing_setup::{RedactingMakeWriter, init_tracing};
