//! The gateway: WSS JSON-RPC, and the five checks every call passes before a handler runs.
//!
//! 1. Verify the access token (signature, expiry) and that the device is still allowed.
//! 2. Rate limit per tenant, user and method, and cap the message size.
//! 3. Check the role, scope and plan the method declares.
//! 4. Idempotency: a replayed write returns its first result.
//! 5. Call the handler with a `RequestCtx`. Row-level security checks scope again in the database.

mod clock;
mod hub;
mod idempotency;
mod pipeline;
mod ratelimit;
mod server;

pub use clock::{Clock, ManualClock, SystemClock};
pub use hub::{Hub, Push};
pub use pipeline::{Gateway, GatewayConfig, Session};
pub use ratelimit::{Limit, Limits, RateLimiter};
pub use server::router;
