//! Permission-related types shared between the sharer and viewer protocols.

use super::{Role, TeamAclData};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum LinkAccessLevelUpdateResponse {
    Ok { role: Option<Role> },
    Error,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum AddGuestsResponse {
    Success,
    Error(FailedToAddGuestsReason),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum FailedToAddGuestsReason {
    /// Unexpected, something went wrong in the server.
    Invalid,
    /// One or more of the emails did not correspond with Warp users.
    NotWarpUsers,
    /// One or more of the guests has already been added to the session.
    GuestAlreadyAdded,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum RemoveGuestResponse {
    Success,
    Error(FailedToRemoveGuestReason),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum FailedToRemoveGuestReason {
    /// Unexpected, something went wrong in the server.
    Invalid,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum UpdatePendingUserRoleResponse {
    Success,
    Error(FailedToUpdatePendingUserRoleReason),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum FailedToUpdatePendingUserRoleReason {
    /// Unexpected, something went wrong in the server.
    Invalid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TeamAccessLevelUpdateResponse {
    Success {
        team_uid: String,
        team_acl: Option<TeamAclData>,
    },
    Error(FailedToUpdateTeamAccessLevelReason),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum FailedToUpdateTeamAccessLevelReason {
    /// Unexpected, something went wrong in the server.
    Invalid,
}
