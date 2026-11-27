use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{DeviceId, ScopeId, TenantId, TraceId, UserId};

/// A member's role inside a tenant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "core/")]
pub enum Role {
    Member,
    DepartmentLead,
    ProductLeader,
    CSuite,
    Admin,
}

/// Entitlement tier checked by the gateway before any server-side agent runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "core/")]
pub enum Plan {
    Starter,
    Team,
    Business,
    Enterprise,
}

/// Whether anything in the current run came from external, untrusted text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "core/")]
pub enum Taint {
    Clean,
    External,
}

impl Taint {
    /// Taint only ever spreads: once external, always external.
    pub fn merge(self, other: Taint) -> Taint {
        if self == Taint::External || other == Taint::External {
            Taint::External
        } else {
            Taint::Clean
        }
    }
}

/// Who is acting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Principal {
    /// A signed-in member.
    User { user_id: UserId },
    /// An agent acting for a member under a task capability that expires with the task.
    Agent {
        on_behalf_of: UserId,
        task_id: uuid::Uuid,
    },
    /// The server itself (sequencer, ambient engine, relay router).
    System,
}

/// Created by the gateway after authentication and passed by value into every call
/// that touches data. Nothing that reads or writes data runs without one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestCtx {
    pub tenant_id: TenantId,
    pub principal: Principal,
    pub roles: Vec<Role>,
    pub scopes: Vec<ScopeId>,
    pub plan: Plan,
    pub taint: Taint,
    pub trace_id: TraceId,
    pub device_id: Option<DeviceId>,
}

impl RequestCtx {
    /// True when the caller may read this scope.
    pub fn can_read(&self, scope: ScopeId) -> bool {
        self.scopes.contains(&scope)
    }

    pub fn has_role(&self, role: Role) -> bool {
        self.roles.contains(&role)
    }

    /// The user the call is for, whether a person or an agent acting for one.
    pub fn user_id(&self) -> Option<UserId> {
        match &self.principal {
            Principal::User { user_id } => Some(*user_id),
            Principal::Agent { on_behalf_of, .. } => Some(*on_behalf_of),
            Principal::System => None,
        }
    }

    /// Returns a copy of this context with taint merged in.
    pub fn tainted_by(mut self, taint: Taint) -> Self {
        self.taint = self.taint.merge(taint);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(scopes: Vec<ScopeId>) -> RequestCtx {
        RequestCtx {
            tenant_id: TenantId::new(),
            principal: Principal::User {
                user_id: UserId::new(),
            },
            roles: vec![Role::Member],
            scopes,
            plan: Plan::Team,
            taint: Taint::Clean,
            trace_id: TraceId::new(),
            device_id: None,
        }
    }

    #[test]
    fn taint_never_clears() {
        assert_eq!(Taint::External.merge(Taint::Clean), Taint::External);
        assert_eq!(Taint::Clean.merge(Taint::External), Taint::External);
        assert_eq!(Taint::Clean.merge(Taint::Clean), Taint::Clean);
    }

    #[test]
    fn can_read_only_listed_scopes() {
        let a = ScopeId::new();
        let c = ctx(vec![a]);
        assert!(c.can_read(a));
        assert!(!c.can_read(ScopeId::new()));
    }

    #[test]
    fn tainted_by_sticks() {
        let c = ctx(vec![])
            .tainted_by(Taint::External)
            .tainted_by(Taint::Clean);
        assert_eq!(c.taint, Taint::External);
    }
}
