//! JSON-RPC types, the method table and the checks that run before any handler.
//!
//! Every method is declared once in `methods.rs` with its caller roles, scope rule, minimum plan and
//! whether it writes. The gateway reads the declaration before calling a handler, `contracts/protocol.md`
//! is generated from the same table, and TypeScript types for the app are exported from these types.

mod authorize;
mod methods;
mod render;
mod rpc;

pub use authorize::authorize;
pub use methods::{Access, Link, METHODS, MethodDecl, PUSHES, PushDecl, ScopeRule, find};
pub use render::{render_app_methods_ts, render_protocol_md};
pub use rpc::{
    NodeInfo, PROTOCOL_VERSION, RpcError, RpcRequest, RpcResponse, SessionState, rpc_error_code,
};
