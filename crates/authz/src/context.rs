//! Builds a `RequestCtx` from the database: the member's role, the scopes they can read right now,
//! and their tenant's plan. The gateway calls this after verifying an access token.
//!
//! Scope rules (README: Access scopes):
//! - a member reads the scopes they belong to;
//! - departments in the same fold share context, so members also read every non-sealed department
//!   in the folds of their non-sealed departments, and the fold itself;
//! - product leaders and the C-suite read every non-sealed scope (God's eye shows sealed scopes as
//!   summaries only, never their records), plus any sealed scope they belong to;
//! - sealed scopes (finance, legal, HR) are readable only by their own members.

use library_core::{
    DeviceId, ErrorCode, LibraryError, Plan, Principal, RequestCtx, Role, ScopeId, Taint, TenantId,
    TraceId, UserId,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

fn denied() -> LibraryError {
    LibraryError::new(ErrorCode::Unauthenticated, "Sign in again to continue.")
}

fn internal(_e: sqlx::Error) -> LibraryError {
    LibraryError::internal()
}

fn parse_role(s: &str) -> Option<Role> {
    Some(match s {
        "member" => Role::Member,
        "department_lead" => Role::DepartmentLead,
        "product_leader" => Role::ProductLeader,
        "c_suite" => Role::CSuite,
        "admin" => Role::Admin,
        _ => return None,
    })
}

fn parse_plan(s: &str) -> Plan {
    match s {
        "team" => Plan::Team,
        "business" => Plan::Business,
        "enterprise" => Plan::Enterprise,
        _ => Plan::Starter,
    }
}

/// Context for a signed-in member on a registered device. Fails with `Unauthenticated` when the
/// membership is gone or the device is unknown, revoked or belongs to someone else.
pub async fn build_ctx(
    auth: &PgPool,
    user: UserId,
    tenant: TenantId,
    device: DeviceId,
) -> Result<RequestCtx, LibraryError> {
    let ok: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM devices WHERE id = $1 AND tenant_id = $2 AND user_id = $3 AND revoked_at IS NULL)")
        .bind(device.0)
        .bind(tenant.0)
        .bind(user.0)
        .fetch_one(auth)
        .await
        .map_err(internal)?;
    if !ok {
        return Err(denied());
    }
    let role: Option<String> =
        sqlx::query_scalar("SELECT role FROM memberships WHERE tenant_id = $1 AND user_id = $2")
            .bind(tenant.0)
            .bind(user.0)
            .fetch_optional(auth)
            .await
            .map_err(internal)?;
    let membership_role = role.as_deref().and_then(parse_role).ok_or_else(denied)?;

    // A scope lead is a department lead even when their membership role is plain member.
    let leads: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM scope_members WHERE tenant_id = $1 AND user_id = $2 AND scope_role = 'lead')")
        .bind(tenant.0)
        .bind(user.0)
        .fetch_one(auth)
        .await
        .map_err(internal)?;
    let mut roles = vec![membership_role];
    if leads && !roles.contains(&Role::DepartmentLead) {
        roles.push(Role::DepartmentLead);
    }

    let sees_all_unsealed = matches!(membership_role, Role::ProductLeader | Role::CSuite);
    let rows = sqlx::query(
        "WITH own AS (
           SELECT s.id, s.sealed, s.fold_id FROM scopes s
           JOIN scope_members m ON m.scope_id = s.id AND m.user_id = $1
           WHERE s.tenant_id = $2 AND m.tenant_id = $2),
         folds AS (SELECT DISTINCT fold_id FROM own WHERE fold_id IS NOT NULL AND NOT sealed)
         SELECT id FROM own
         UNION
         SELECT s.id FROM scopes s
           WHERE s.tenant_id = $2 AND NOT s.sealed
             AND (s.fold_id IN (SELECT fold_id FROM folds) OR s.id IN (SELECT fold_id FROM folds))
         UNION
         SELECT s.id FROM scopes s WHERE s.tenant_id = $2 AND NOT s.sealed AND $3",
    )
    .bind(user.0)
    .bind(tenant.0)
    .bind(sees_all_unsealed)
    .fetch_all(auth)
    .await
    .map_err(internal)?;
    let mut scopes: Vec<ScopeId> = rows
        .iter()
        .map(|r| ScopeId(r.get::<Uuid, _>("id")))
        .collect();
    scopes.sort();

    let plan: Option<String> = sqlx::query_scalar("SELECT plan FROM plans WHERE tenant_id = $1")
        .bind(tenant.0)
        .fetch_optional(auth)
        .await
        .map_err(internal)?;

    Ok(RequestCtx {
        tenant_id: tenant,
        principal: Principal::User { user_id: user },
        roles,
        scopes,
        plan: plan.as_deref().map(parse_plan).unwrap_or(Plan::Starter),
        taint: Taint::Clean,
        trace_id: TraceId::new(),
        device_id: Some(device),
    })
}
