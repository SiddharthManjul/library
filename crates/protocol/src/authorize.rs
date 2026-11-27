use library_core::{ErrorCode, LibraryError, RequestCtx, ScopeId};

use crate::methods::{Access, MethodDecl, ScopeRule};

/// The gateway's check before any handler runs: role, plan, scope. Returns the first failure.
/// `scope` is the `scope_id` from the call's params, when the method names one.
///
/// Row-level security checks the same things again in the database; this is never the only check.
pub fn authorize(
    method: &MethodDecl,
    ctx: &RequestCtx,
    scope: Option<ScopeId>,
) -> Result<(), LibraryError> {
    match method.access {
        Access::Public => return Ok(()),
        Access::Server => {
            return Err(LibraryError::new(
                ErrorCode::PermissionDenied,
                "This method is not available to devices.",
            ));
        }
        Access::Member => {}
        Access::Roles(roles) => {
            if !roles.iter().any(|r| ctx.has_role(*r)) {
                return Err(LibraryError::new(
                    ErrorCode::PermissionDenied,
                    "Your role cannot do this.",
                ));
            }
        }
    }
    if ctx.plan < method.plan {
        return Err(LibraryError::new(
            ErrorCode::PlanRequired,
            "Your plan does not include this.",
        ));
    }
    if method.scope == ScopeRule::Own {
        let scope = scope
            .ok_or_else(|| LibraryError::new(ErrorCode::InvalidParams, "A scope is required."))?;
        if !ctx.can_read(scope) {
            return Err(LibraryError::new(
                ErrorCode::ScopeDenied,
                "You do not have access to this scope.",
            ));
        }
    }
    Ok(())
}
