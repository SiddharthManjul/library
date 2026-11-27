//! Shared types, traits, `RequestCtx` and error codes.
//! Every other crate depends on this one; it depends on no other library crate.

mod ctx;
mod device;
mod error;
mod ids;
mod traits;

pub use ctx::{Plan, Principal, RequestCtx, Role, Taint};
pub use device::{DEVICE_PROOF_DOMAIN, DeviceRegistration, device_registration_message};
pub use error::{ErrorCode, LibraryError, Result};
pub use ids::{DeviceId, ScopeId, TenantId, TraceId, UserId};
pub use traits::{AuditRecord, AuditSink, BoxFuture, Call, RpcHandler};
