//! A generated rejection test per method (README: API, "Every method declares its role, scope and plan").
#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not

use library_core::{
    ErrorCode, Plan, Principal, RequestCtx, Role, ScopeId, Taint, TenantId, TraceId, UserId,
};
use library_protocol::{Access, METHODS, ScopeRule, authorize, find};

fn ctx(roles: Vec<Role>, plan: Plan, scopes: Vec<ScopeId>) -> RequestCtx {
    RequestCtx {
        tenant_id: TenantId::new(),
        principal: Principal::User {
            user_id: UserId::new(),
        },
        roles,
        scopes,
        plan,
        taint: Taint::Clean,
        trace_id: TraceId::new(),
        device_id: None,
    }
}

#[test]
fn method_names_are_unique_and_well_formed() {
    let mut seen = std::collections::HashSet::new();
    for m in METHODS {
        assert!(seen.insert(m.name), "duplicate method {}", m.name);
        assert!(
            m.name
                .split('.')
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_lowercase() || c == '_')),
            "{}",
            m.name
        );
        assert!(!m.purpose.is_empty());
    }
    assert!(METHODS.len() >= 60);
}

#[test]
fn every_method_rejects_a_caller_without_the_role() {
    for m in METHODS {
        if let Access::Roles(roles) = m.access {
            let scope = ScopeId::new();
            // A member who holds none of the listed roles, with the best plan and the scope.
            let others: Vec<Role> = [
                Role::Member,
                Role::DepartmentLead,
                Role::ProductLeader,
                Role::CSuite,
                Role::Admin,
            ]
            .into_iter()
            .filter(|r| !roles.contains(r))
            .collect();
            let c = ctx(others, Plan::Enterprise, vec![scope]);
            assert_eq!(
                authorize(m, &c, Some(scope)).unwrap_err().code,
                ErrorCode::PermissionDenied,
                "{}",
                m.name
            );
        }
    }
}

#[test]
fn every_method_rejects_a_plan_that_is_too_low() {
    for m in METHODS
        .iter()
        .filter(|m| m.plan > Plan::Starter && !matches!(m.access, Access::Public | Access::Server))
    {
        let scope = ScopeId::new();
        let roles = match m.access {
            Access::Roles(r) => r.to_vec(),
            _ => vec![Role::Member],
        };
        let lower = match m.plan {
            Plan::Team => Plan::Starter,
            Plan::Business => Plan::Team,
            _ => Plan::Business,
        };
        let c = ctx(roles, lower, vec![scope]);
        assert_eq!(
            authorize(m, &c, Some(scope)).unwrap_err().code,
            ErrorCode::PlanRequired,
            "{}",
            m.name
        );
    }
}

#[test]
fn every_scoped_method_rejects_a_missing_or_foreign_scope() {
    for m in METHODS
        .iter()
        .filter(|m| m.scope == ScopeRule::Own && !matches!(m.access, Access::Server))
    {
        let roles = match m.access {
            Access::Roles(r) => r.to_vec(),
            _ => vec![Role::Member],
        };
        let held = ScopeId::new();
        let c = ctx(roles, Plan::Enterprise, vec![held]);
        assert_eq!(
            authorize(m, &c, Some(ScopeId::new())).unwrap_err().code,
            ErrorCode::ScopeDenied,
            "{}",
            m.name
        );
        assert_eq!(
            authorize(m, &c, None).unwrap_err().code,
            ErrorCode::InvalidParams,
            "{}",
            m.name
        );
        assert!(authorize(m, &c, Some(held)).is_ok(), "{}", m.name);
    }
}

#[test]
fn server_only_methods_are_never_callable_by_a_device() {
    let all = ctx(
        vec![
            Role::Admin,
            Role::CSuite,
            Role::ProductLeader,
            Role::DepartmentLead,
            Role::Member,
        ],
        Plan::Enterprise,
        vec![],
    );
    for m in METHODS
        .iter()
        .filter(|m| matches!(m.access, Access::Server))
    {
        assert!(authorize(m, &all, None).is_err(), "{}", m.name);
    }
    assert!(find("brain.context").is_some());
}

#[test]
fn a_fully_entitled_caller_is_allowed() {
    let scope = ScopeId::new();
    let c = ctx(vec![Role::Admin], Plan::Enterprise, vec![scope]);
    assert!(authorize(find("org.departments.upsert").unwrap(), &c, None).is_ok());
    assert!(authorize(find("connections.start").unwrap(), &c, Some(scope)).is_ok());
}

#[test]
fn error_codes_map_to_distinct_rpc_numbers() {
    use ErrorCode::*;
    let all = [
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
    ];
    let nums: std::collections::HashSet<i32> = all
        .iter()
        .map(|c| library_protocol::rpc_error_code(*c))
        .collect();
    assert_eq!(nums.len(), all.len());
}
