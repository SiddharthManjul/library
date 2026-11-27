use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Every error code the protocol can return. One enum, one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "core/")]
pub enum ErrorCode {
    Unauthenticated,
    TokenExpired,
    PermissionDenied,
    ScopeDenied,
    PlanRequired,
    RateLimited,
    PayloadTooLarge,
    InvalidParams,
    NotFound,
    Conflict,
    VersionMismatch,
    ApprovalRequired,
    Unavailable,
    Internal,
}

impl ErrorCode {
    /// Whether a client may retry the same call unchanged.
    pub fn retryable(self) -> bool {
        matches!(
            self,
            ErrorCode::RateLimited | ErrorCode::Unavailable | ErrorCode::Conflict
        )
    }
}

/// What leaves the server: a code, a message safe to show a user, and a retry hint.
/// Never contains SQL, file paths, stack traces, secrets or untrusted text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error, TS)]
#[error("{code:?}: {message}")]
#[ts(export, export_to = "core/")]
pub struct LibraryError {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
}

impl LibraryError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            retryable: code.retryable(),
            code,
            message: message.into(),
        }
    }

    /// Internal failures hide their cause from the caller.
    pub fn internal() -> Self {
        Self::new(
            ErrorCode::Internal,
            "Something went wrong on our side. Try again in a moment.",
        )
    }
}

pub type Result<T, E = LibraryError> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_follows_code() {
        assert!(LibraryError::new(ErrorCode::RateLimited, "slow down").retryable);
        assert!(!LibraryError::new(ErrorCode::ScopeDenied, "no").retryable);
    }

    #[test]
    fn internal_error_leaks_nothing() {
        let e = LibraryError::internal();
        assert!(!e.message.to_lowercase().contains("sql"));
    }
}
