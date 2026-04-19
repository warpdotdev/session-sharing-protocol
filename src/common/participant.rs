use super::{InputReplicaId, Role, Selection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum ParticipantType {
    Sharer,
    Viewer { role: Role },
}

/// An ID for a shared session participant that is unique across all participants across all shared sessions.
/// If a viewer joins a shared session multiple times from the same machine, they are treated as separate participants with their own ParticipantId.
/// A participant reconnecting should keep the same participant ID.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ParticipantId(String);

impl From<String> for ParticipantId {
    fn from(value: String) -> Self {
        ParticipantId(value)
    }
}

impl ParticipantId {
    pub fn new() -> ParticipantId {
        ParticipantId(Uuid::new_v4().to_string())
    }
}

impl Default for ParticipantId {
    fn default() -> Self {
        ParticipantId::new()
    }
}

impl std::fmt::Display for ParticipantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

/// Mostly static information about a participant.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ProfileData {
    pub firebase_uid: String,
    pub display_name: String,

    /// If None, the client should render an avatar themselves.
    pub photo_url: Option<String>,
    pub email: Option<String>,

    pub input_replica_id: InputReplicaId,
}

/// Contains information about a shared session participant.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ParticipantInfo {
    pub id: ParticipantId,
    pub profile_data: ProfileData,
    pub selection: Selection,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Sharer {
    pub info: ParticipantInfo,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Viewer {
    pub info: ParticipantInfo,
    pub role: Role,
    /// Whether or not this viewer is still part of the session.
    pub is_present: bool,
}

/// Information about a viewer that is still part of the session.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PresentViewer {
    pub info: ParticipantInfo,
    /// The maximum access level this viewer has been given.
    pub max_acl: Role,
}

/// Information about a viewer that is no longer a part of the session.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AbsentViewer {
    pub info: ParticipantInfo,
}

/// Information about a user who is a direct guest on the session.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Guest {
    pub profile_data: ProfileData,
    /// The direct access level that this guest has been given.
    pub direct_acl: Role,
}

/// Information about a non-Warp user who has been invited to the session.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PendingGuest {
    pub email: String,
    /// The direct access level that this guest has been given.
    pub direct_acl: Role,
}

/// Information about the full list of all participants in a shared session.
///
/// To derive the session's direct guests, all of the users with direct acls
/// across `present_viewers`, `absent_viewers`, and `non_viewer_guests` must be
/// accumulated.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ParticipantList {
    pub sharer: Sharer,

    /// Legacy field kept for backwards compatibility. After the ACL transition
    /// we should be using `present_viewers`, `absent_viewers`, and
    /// `non_viewer_guests` and can remove this field.
    pub viewers: Vec<Viewer>,

    /// Viewers that are currently on the session.
    pub present_viewers: Vec<PresentViewer>,
    /// Viewers that are no longer on the session.
    pub absent_viewers: Vec<AbsentViewer>,
    /// Users that have a direct ACL on the session.
    pub guests: Vec<Guest>,
    /// Non-Warp users who have been invited to the session.
    pub pending_guests: Vec<PendingGuest>,
}

impl ParticipantList {
    /// Downgrades all `Role::Full` fields to `Role::Executor`.
    /// Used for backward compatibility with clients that don't support the Full role.
    pub fn downgrade_full_roles(&mut self) {
        for viewer in &mut self.viewers {
            viewer.role.downgrade_full();
        }
        for viewer in &mut self.present_viewers {
            viewer.max_acl.downgrade_full();
        }
        for guest in &mut self.guests {
            guest.direct_acl.downgrade_full();
        }
        for guest in &mut self.pending_guests {
            guest.direct_acl.downgrade_full();
        }
    }
}

/// Information received from fetching and processing the participant list.
pub struct ParticipantListInfo {
    /// The full list of participants in a shared session.
    pub list: ParticipantList,
    /// The list of present participants who do not have access to the session.
    pub viewers_without_access: Vec<ParticipantId>,
}
