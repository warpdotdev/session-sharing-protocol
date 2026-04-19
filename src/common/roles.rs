use super::ParticipantId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Role {
    Reader,
    Executor,
    /// Executor, and can change ACLs of others
    Full,
}

impl Role {
    /// Returns true if this role has execution permissions.
    pub fn can_execute(&self) -> bool {
        matches!(self, Role::Executor | Role::Full)
    }

    /// Downgrades `Full` to `Executor` for clients that don't support the Full role.
    pub fn downgrade_full(&mut self) {
        if *self == Role::Full {
            *self = Role::Executor;
        }
    }
}

impl Default for Role {
    fn default() -> Self {
        Self::Reader
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
/// Info about different types of ACLs for a user.
pub struct AccessLevels {
    /// The maximum ACL given to the user, could be direct, link-based, etc.
    pub max_acl: Role,
    /// The direct ACL given to the user.
    pub direct_acl: Option<Role>,
}

/// An ID for a role request that is unique across all participants across all shared sessions.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RoleRequestId(String);

impl From<String> for RoleRequestId {
    fn from(value: String) -> Self {
        RoleRequestId(value)
    }
}

impl RoleRequestId {
    pub fn new() -> RoleRequestId {
        RoleRequestId(Uuid::new_v4().to_string())
    }
}

impl Default for RoleRequestId {
    fn default() -> Self {
        RoleRequestId::new()
    }
}

impl std::fmt::Display for RoleRequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RoleRequestRejectedReason {
    RejectedBySharer,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RoleRequestResponse {
    Approved { new_role: Role },
    Rejected { reason: RoleRequestRejectedReason },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PendingRoleRequest {
    participant_id: ParticipantId,
    request_id: RoleRequestId,
    role: Role,
}
