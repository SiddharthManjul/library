use library_core::{ErrorCode, LibraryError, TenantId, UserId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Version of the wire protocol. A node and a gateway on different versions refuse to talk.
pub const PROTOCOL_VERSION: u32 = 1;

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "protocol/")]
pub struct RpcRequest {
    pub jsonrpc: String,
    #[ts(type = "number")]
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC 2.0 error object. `data` carries the typed library error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "protocol/")]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<LibraryError>,
}

/// JSON-RPC 2.0 response: exactly one of `result` and `error`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "protocol/")]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[ts(type = "number")]
    pub id: u64,
    pub result: Option<serde_json::Value>,
    pub error: Option<RpcError>,
}

impl RpcResponse {
    pub fn ok(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: u64, e: LibraryError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(RpcError {
                code: rpc_error_code(e.code),
                message: e.message.clone(),
                data: Some(e),
            }),
        }
    }
}

impl RpcResponse {
    /// JSON-RPC 2.0 reserves -32700 (parse error) and -32601 (method not found).
    pub fn parse_error() -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: 0,
            result: None,
            error: Some(RpcError {
                code: -32700,
                message: "The message was not valid JSON-RPC.".into(),
                data: None,
            }),
        }
    }

    pub fn method_not_found(id: u64) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(RpcError {
                code: -32601,
                message: "This method does not exist.".into(),
                data: None,
            }),
        }
    }
}

/// Stable JSON-RPC error numbers in the implementation-defined range (-32000 to -32099).
pub fn rpc_error_code(code: ErrorCode) -> i32 {
    match code {
        ErrorCode::InvalidParams => -32602,
        ErrorCode::Unauthenticated => -32001,
        ErrorCode::TokenExpired => -32002,
        ErrorCode::PermissionDenied => -32003,
        ErrorCode::ScopeDenied => -32004,
        ErrorCode::PlanRequired => -32005,
        ErrorCode::RateLimited => -32006,
        ErrorCode::PayloadTooLarge => -32007,
        ErrorCode::NotFound => -32008,
        ErrorCode::Conflict => -32009,
        ErrorCode::VersionMismatch => -32010,
        ErrorCode::ApprovalRequired => -32011,
        ErrorCode::Unavailable => -32012,
        ErrorCode::Internal => -32603,
    }
}

/// Answer to `node.info`: who the app is talking to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "protocol/")]
pub struct NodeInfo {
    pub name: String,
    pub version: String,
    pub protocol: u32,
    pub platform: String,
}

/// Answer to `session.state`: whether a member is signed in on this device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "snake_case")]
#[ts(export, export_to = "protocol/")]
pub enum SessionState {
    SignedOut,
    SignedIn {
        #[ts(type = "string")]
        tenant_id: TenantId,
        #[ts(type = "string")]
        user_id: UserId,
    },
}
