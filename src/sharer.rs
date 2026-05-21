//! The message types that are communicated between the
//! server and a sharer client.
//!
//! When a client wants to create a shared session, the client
//! will make a request against /sessions/create. The client
//! must then send an [`Initialize`] message with the relevant data
//! to start the shared session. If successful, the server
//! will acknowledge the creation of the shared
//! session via the [`SessionInitialized`] message.
//!
//! Remember to annotate #[serde(default)] to every new field added for backward compatibility,
//! since old clients may not specify new fields expected by the server.

use crate::common::{
    ActivePrompt, ActivePromptUpdate, AgentPromptFailureReason, AgentPromptRequest,
    AgentPromptRequestId, BlockId, BufferId, CommandExecutionFailureReason,
    CommandExecutionRequestId, ControlAction, ControlActionFailureReason, ControlActionRequestId,
    FeatureSupport, InputOperationId, InputReplicaId, InputUpdate, InputUpdateFailureReason,
    OrderedTerminalEvent, ParticipantId, ParticipantList, ParticipantPresenceUpdate, Role,
    RoleRequestId, RoleRequestResponse, Selection, SelectionUpdate, SessionId, SessionSecret,
    TelemetryContext, UniversalDeveloperInputContext, UniversalDeveloperInputContextUpdate, UserID,
    WindowSize, WriteToPtyFailureReason, WriteToPtyRequestId,
};

use super::common::Scrollback;
use byte_unit::Byte;
use serde::ser::SerializeStructVariant;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// Possible reasons why the server might gracefully terminate
/// a shared session.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum SessionTerminatedReason {
    /// Unknown error occurred. Session cannot continue.
    InternalServerError {
        /// Details about what happened. This should
        /// 1. only be provided to the sharer client,
        /// 2. not necessarily be user-facing, and
        /// 3. clients should _not_ try to match on the exact message
        details: String,
    },
    /// The session exceeded its size limit.
    ExceededSizeLimit,
    /// The user does not have any more quota remaining.
    NoUserQuotaRemaining {
        // This is left as an empty struct to make it
        // easier to add fields (e.g. next refresh time)
        // in the future in a backwards-compatible way.
    },
}

impl SessionTerminatedReason {
    pub fn internal_server_error(details: impl Into<String>) -> Self {
        Self::InternalServerError {
            details: details.into(),
        }
    }
    pub fn internal_server_error_without_details() -> Self {
        Self::internal_server_error(String::new())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
/// Client-side reasons for ending the shared session.
pub enum SessionEndedReason {
    /// The session was ended gracefully.
    EndedBySharer,
    /// The sharer was idle for too long.
    InactivityLimitReached,
    /// The session exceeded its size limit.
    // TODO: remove as part of quota enforcement work
    ExceededSizeLimit,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ReconnectionFailedReason {
    /// Unexpected, means something went wrong in the server.
    Invalid,
    /// The session with the specified ID does not exist.
    SessionNotFound,
    /// The specified password was incorrect.
    WrongPassword,
    /// The specified reconnection token was incorrect.
    WrongReconnectionToken,
    /// The firebase ID of the sharer was missing or doesn't match the original one.
    WrongFirebaseUid,
    /// The sharer does not have any remaining quota.
    NoUserQuotaRemaining,
    /// The session is not accessible.
    SessionNotAccessible,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy)]
pub enum RoleUpdateReason {
    #[default]
    UpdatedBySharer,
    InactivityLimitReached,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum QuotaType {
    BytesUsed,
    SessionsCreated,
}

/// The reasons we might fail to initialize a new session.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum FailedToInitializeSessionReason {
    /// The scrollback exceeds the user's quota.
    ScrollbackTooLarge {
        // This is left as an empty struct to make it
        // easier to add fields (e.g. remaining scrollback size)
        // in the future in a backwards-compatible way.
    },
    /// The sharer does not have any remaining quota.
    NoUserQuotaRemaining { quota_type: QuotaType },
    /// The sharer could not be attributed to a Warp user.
    UserNotFound,
    /// Something unexpectedly went wrong.
    InternalServerError {
        /// Details about what happened. This should
        /// not necessarily be user-facing, and clients should
        /// _not_ try to match on the exact message.
        details: String,
    },
}

impl FailedToInitializeSessionReason {
    pub fn internal_server_error_without_details() -> Self {
        FailedToInitializeSessionReason::InternalServerError {
            details: String::new(),
        }
    }
}

// Permission response types are now in common::permissions.
// Re-exported here for backward compatibility.
pub use crate::common::{
    AddGuestsResponse, FailedToAddGuestsReason, FailedToRemoveGuestReason,
    FailedToUpdatePendingUserRoleReason, FailedToUpdateTeamAccessLevelReason,
    LinkAccessLevelUpdateResponse, RemoveGuestResponse, TeamAccessLevelUpdateResponse,
    UpdatePendingUserRoleResponse,
};

#[derive(Clone, Debug)]
pub enum SessionSourceType {
    /// The session was started by a user directly. `task_id` is the
    /// server-side `ai_tasks` row id when known.
    User { task_id: Option<String> },
    /// The session was started in the course of spinning up an ambient agent.
    AmbientAgent { task_id: Option<String> },
}

// `#[derive(Default)]` with `#[default]` on a struct variant is not yet
// supported on stable Rust, so we implement `Default` manually.
impl Default for SessionSourceType {
    fn default() -> Self {
        SessionSourceType::User { task_id: None }
    }
}

impl SessionSourceType {
    /// Returns the `task_id` carried by this source type, regardless of variant.
    pub fn orchestrator_task_id(&self) -> Option<&str> {
        match self {
            Self::User { task_id } | Self::AmbientAgent { task_id } => task_id.as_deref(),
        }
    }
}

/// Internal helper that mirrors all wire representations of SessionSourceType
/// (both legacy and new) so we don't recursively call SessionSourceType's
/// custom Deserialize impl.
#[derive(Deserialize)]
#[serde(untagged)]
enum SessionSourceTypeWire {
    /// Legacy representation: bare `"User"` or `"AmbientAgent"`.
    Legacy(LegacySessionSourceType),
    /// New representation: externally tagged `User` with fields, e.g.
    /// `{ "User": { "task_id": "..." } }`.
    NewUser {
        #[serde(rename = "User")]
        user: TaskIdFields,
    },
    /// New representation: externally tagged `AmbientAgent` with fields, e.g.
    /// `{ "AmbientAgent": { "task_id": "..." } }`.
    NewAmbientAgent {
        #[serde(rename = "AmbientAgent")]
        ambient_agent: TaskIdFields,
    },
}

#[derive(Deserialize)]
struct TaskIdFields {
    #[serde(default)]
    task_id: Option<String>,
}

impl From<SessionSourceTypeWire> for SessionSourceType {
    fn from(value: SessionSourceTypeWire) -> Self {
        match value {
            SessionSourceTypeWire::Legacy(LegacySessionSourceType::User) => {
                SessionSourceType::User { task_id: None }
            }
            SessionSourceTypeWire::Legacy(LegacySessionSourceType::AmbientAgent) => {
                SessionSourceType::AmbientAgent { task_id: None }
            }
            SessionSourceTypeWire::NewUser {
                user: TaskIdFields { task_id },
            } => SessionSourceType::User { task_id },
            SessionSourceTypeWire::NewAmbientAgent {
                ambient_agent: TaskIdFields { task_id },
            } => SessionSourceType::AmbientAgent { task_id },
        }
    }
}

impl<'de> Deserialize<'de> for SessionSourceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SessionSourceTypeWire::deserialize(deserializer)?;
        Ok(SessionSourceType::from(wire))
    }
}

/// Emits the bare legacy form when `task_id` is `None` and the struct form
/// otherwise, so older readers that only understand the unit-variant shape
/// stay forward-compatible until they pick up the new deserializer.
impl Serialize for SessionSourceType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            SessionSourceType::User { task_id: None } => {
                serializer.serialize_unit_variant("SessionSourceType", 0, "User")
            }
            SessionSourceType::User {
                task_id: Some(task_id),
            } => {
                let mut sv =
                    serializer.serialize_struct_variant("SessionSourceType", 0, "User", 1)?;
                sv.serialize_field("task_id", task_id)?;
                sv.end()
            }
            SessionSourceType::AmbientAgent { task_id: None } => {
                serializer.serialize_unit_variant("SessionSourceType", 1, "AmbientAgent")
            }
            SessionSourceType::AmbientAgent {
                task_id: Some(task_id),
            } => {
                let mut sv = serializer.serialize_struct_variant(
                    "SessionSourceType",
                    1,
                    "AmbientAgent",
                    1,
                )?;
                sv.serialize_field("task_id", task_id)?;
                sv.end()
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum LegacySessionSourceType {
    #[default]
    User,
    AmbientAgent,
}

impl From<&SessionSourceType> for LegacySessionSourceType {
    fn from(value: &SessionSourceType) -> Self {
        match value {
            SessionSourceType::User { .. } => LegacySessionSourceType::User,
            SessionSourceType::AmbientAgent { .. } => LegacySessionSourceType::AmbientAgent,
        }
    }
}

/// Configures the lifetime of the session after sharing ends.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub enum Lifetime {
    /// The session is deleted immediately when sharing ends.
    #[default]
    Ephemeral,
    /// The session persists after sharing ends.
    ///
    /// It is not specified how long a lingering session is available for after it ends.
    /// Currently, all session contents expire after one week, but this is a server implementation
    /// detail that clients must not rely on. In the future, we may expose a lifetime option that
    /// includes a client-provided TTL.
    Lingering,
}

/// The initial state that the sharer must supply when starting
/// a shared session.
#[derive(Debug, Deserialize, Serialize)]
pub struct InitPayload {
    pub scrollback: Scrollback,

    pub active_prompt: ActivePrompt,

    pub window_size: WindowSize,

    pub user_id: UserID,

    /// What the sharer currently has selected for presence.
    pub selection: Selection,

    pub init_block_id: BlockId,

    pub input_replica_id: InputReplicaId,

    pub telemetry_context: Option<TelemetryContext>,

    #[serde(default)]
    pub lifetime: Lifetime,

    /// The universal developer input context state.
    #[serde(default)]
    pub universal_developer_input_context: Option<UniversalDeveloperInputContext>,

    /// The source type for this shared session (i.e. user or ambient agent).
    #[serde(default)]
    pub source_type: SessionSourceType,

    /// Client feature support declaration.
    #[serde(default)]
    pub feature_support: FeatureSupport,
}

/// The reconnection token for a shared session.
/// A sharer must specify this to reconnect to the session and resume sharing.
/// The client should treat this as some opaque string.
#[derive(Hash, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(transparent)]
pub struct ReconnectToken(String);
impl ReconnectToken {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ReconnectToken {
    fn default() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

/// Override the Display impl for the token to return a mask.
/// This makes it harder to leak the token by accident.
impl std::fmt::Display for ReconnectToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***")
    }
}

/// Override the Debug impl for the token to return a mask.
/// This makes it harder to leak the token by accident.
impl std::fmt::Debug for ReconnectToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***")
    }
}

/// The `warp` server framework uses [`FromStr`] to deserialize
/// the string from the route.
impl std::str::FromStr for ReconnectToken {
    type Err = core::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        String::from_str(s).map(ReconnectToken)
    }
}

/// Payload sharer must supply to reconnect to an existing shared session
/// when the websocket was terminated by the server.
#[derive(Debug, Deserialize, Serialize)]
pub struct ReconnectPayload {
    // TODO: Remove in favour of ACLs
    pub session_secret: SessionSecret,
    pub reconnect_token: ReconnectToken,

    pub user_id: UserID,

    /// The ID of the latest block when reconnecting.
    /// This allows the sharer to catch up on any
    /// missed input updates while they were disconnected.
    pub latest_block_id: BlockId,

    /// What the sharer currently has selected for presence.
    pub selection: Selection,

    /// Client feature support declaration.
    #[serde(default)]
    pub feature_support: FeatureSupport,
}

/// The possible messages sent from server to client (sharer).
#[derive(Deserialize, Serialize)]
pub enum DownstreamMessage {
    /// The server sends this message when the session was successfully created.
    SessionInitialized {
        session_id: SessionId,
        // TODO: Remove in favour of ACLs
        session_secret: SessionSecret,
        reconnect_token: ReconnectToken,
        /// The ID assigned to the sharer
        sharer_id: ParticipantId,
        /// The Firebase UID assigned to the sharer.
        sharer_firebase_uid: String,
    },

    /// The server denied the initialization request. No further messages will be processed.
    FailedToInitializeSession {
        reason: FailedToInitializeSessionReason,
    },

    /// The session was terminated. No further messages
    /// will be processed.
    SessionTerminated { reason: SessionTerminatedReason },

    /// The server accepted the reconnection request.
    SessionReconnected {
        /// The last event no received by the server.
        /// The sharer can use this to update the server with any newer events created while disconnected.
        last_received_event_no: Option<usize>,
        participant_list: ParticipantList,
    },

    /// The server denied the reconnection request. No further messages will be processed.
    FailedToReconnect { reason: ReconnectionFailedReason },

    /// The server sends this to confirm it has fully processed events up to the latest_processed_event_no,
    /// and the sharer can safely remove them from memory.
    EventsProcessedAck { latest_processed_event_no: usize },

    /// Sent when the list of participants in the shared session changes.
    ParticipantListUpdated(ParticipantList),

    /// Sent when a participant's presence changes.
    ParticipantPresenceUpdated(ParticipantPresenceUpdate),

    /// The participant (identified by `participant_id`) requested the `role` role.
    RoleRequested {
        participant_id: ParticipantId,
        request_id: RoleRequestId,
        role: Role,
    },

    /// The participant (identified by `participant_id`) has cancelled their role request.
    RoleRequestCancelled {
        participant_id: ParticipantId,
        request_id: RoleRequestId,
    },

    /// A participant's (identified by `participant_id`) role was updated.
    ParticipantRoleChanged {
        participant_id: ParticipantId,
        role: Role,
    },

    /// A participant requested a control action (e.g. cancel conversation).
    ControlActionRequested {
        participant_id: ParticipantId,
        request_id: ControlActionRequestId,
        action: ControlAction,
    },

    /// The input was updated by a participant.
    /// When we receive our own update, we can treat it as an ack.
    InputUpdated(InputUpdate),

    /// The rejection was successfully applied (does not need to be retried by the client).
    InputUpdateRejectedAck { id: InputOperationId },

    /// A participant requested that the given `command` be run in the given buffer.
    CommandExecutionRequested {
        id: CommandExecutionRequestId,
        participant_id: ParticipantId,
        buffer_id: BufferId,
        command: String,
    },

    /// A participant requested to write to the pty, specifically for a long running command.
    WriteToPtyRequested {
        id: WriteToPtyRequestId,
        bytes: Vec<u8>,
    },

    /// A participant requested to send an agent prompt.
    AgentPromptRequested {
        id: AgentPromptRequestId,
        participant_id: ParticipantId,
        request: AgentPromptRequest,
    },

    /// The sharer's link access level update request was responded to.
    LinkAccessLevelUpdateResponse(LinkAccessLevelUpdateResponse),

    /// The request to add guests was responded to.
    AddGuestsResponse(AddGuestsResponse),

    /// The request to remove a guest was responded to.
    RemoveGuestResponse(RemoveGuestResponse),

    /// The request to update a pending user role was responded to.
    UpdatePendingUserRoleResponse(UpdatePendingUserRoleResponse),

    /// The sharer's team access level update request was responsed to.
    TeamAccessLevelUpdateResponse(TeamAccessLevelUpdateResponse),

    /// Update to the universal developer input context from sharer or editor viewers.
    UniversalDeveloperInputContextUpdated(UniversalDeveloperInputContextUpdate),

    /// A viewer reported its terminal size.
    /// Used for remote-control sessions where the viewer's viewport should drive the PTY size.
    ViewerTerminalSizeReported {
        participant_id: ParticipantId,
        window_size: WindowSize,
    },

    /// A response to a [`UpstreamMessage::Ping`].
    /// Used to demonstrate that the server is still alive.
    Pong { data: Vec<u8> },
}

impl DownstreamMessage {
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

/// The possible messages sent from client (sharer) to server.
#[derive(Debug, Deserialize, Serialize)]
pub enum UpstreamMessage {
    /// The client sends this message to start a shared session.
    /// supplying any necessary initial state.
    /// TODO: add size info, etc.
    Initialize(InitPayload),

    /// A heartbeat message to demonstrate that the
    /// client is still alive.
    Ping { data: Vec<u8> },

    /// The client sends this message to explicitly end a session
    /// and notify viewers before the websocket closes.
    EndSession { reason: SessionEndedReason },

    /// Update to the sharer's active prompt.
    UpdateActivePrompt(ActivePromptUpdate),

    /// Update to the universal developer input context (model selection, etc.).
    UpdateUniversalDeveloperInputContext(UniversalDeveloperInputContextUpdate),

    /// Sent when there is any ordered terminal event.
    OrderedTerminalEvent(OrderedTerminalEvent),

    /// Sent to reconnect to the server after disconnection.
    Reconnect(ReconnectPayload),

    /// Sent when the sharer changes what they have selected.
    UpdateSelection(SelectionUpdate),

    /// Changes the participant's (identified by `participant_id`) role.
    UpdateRole {
        participant_id: ParticipantId,
        role: Role,
    },

    /// Changes the user's role (applied to all participants with the same UID).
    UpdateUserRole { user_uid: String, role: Role },

    /// Changes the pending user's role (applied to all participants with the same UID).
    UpdatePendingUserRole { email: String, role: Role },

    /// Responds to the participant's (identified by `participant_id`) role request.
    RespondToRoleRequest {
        participant_id: ParticipantId,
        request_id: RoleRequestId,
        response: RoleRequestResponse,
    },

    /// Updates all participants' roles to be [Role::Reader].
    UpdateAllRolesToReader { reason: RoleUpdateReason },

    /// The sharer updated the input.
    UpdateInput(InputUpdate),

    /// The given operation should be undone on all participants.
    RejectInputUpdate {
        id: InputOperationId,
        reason: InputUpdateFailureReason,
    },

    /// The given command execution request was denied for the specified `reason`.
    RejectCommandExecutionRequest {
        id: CommandExecutionRequestId,
        participant_id: ParticipantId,
        reason: CommandExecutionFailureReason,
    },

    /// The given write to pty request was denied for the specified `reason`.
    RejectWriteToPtyRequest {
        id: WriteToPtyRequestId,
        reason: WriteToPtyFailureReason,
    },

    /// The given agent prompt request was denied for the specified `reason`.
    RejectAgentPromptRequest {
        id: AgentPromptRequestId,
        participant_id: ParticipantId,
        reason: AgentPromptFailureReason,
    },

    /// The given control action request was denied for the specified `reason`.
    RejectControlActionRequest {
        participant_id: ParticipantId,
        request_id: ControlActionRequestId,
        reason: ControlActionFailureReason,
    },

    /// The sharer updated the session's link permissions.
    UpdateLinkAccessLevel { role: Option<Role> },

    /// The sharer updated the session's team permissions.
    UpdateTeamAccessLevel {
        team_uid: String,
        role: Option<Role>,
    },

    /// The sharer added users as session guests by email.
    AddGuests { emails: Vec<String>, role: Role },

    /// The sharer removed a user as a session guest.
    RemoveGuest { user_uid: String },

    /// The sharer removed a pending user as a session guest.
    RemovePendingGuest { email: String },
}

impl UpstreamMessage {
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    pub fn num_bytes(&self) -> Byte {
        match self {
            UpstreamMessage::Initialize(init_payload) => init_payload.scrollback.num_bytes(),
            UpstreamMessage::OrderedTerminalEvent(ordered_terminal_event) => {
                ordered_terminal_event.num_bytes()
            }
            UpstreamMessage::UpdateInput(input_update) => input_update.num_bytes(),
            _ => Byte::from_u64(0),
        }
    }
}

#[cfg(test)]
mod session_source_type_tests {
    //! Wire-compatibility tests for `SessionSourceType`.
    use super::*;

    // --- Deserialization ---

    #[test]
    fn deserialize_legacy_user_bare() {
        let v: SessionSourceType = serde_json::from_str("\"User\"").unwrap();
        assert!(matches!(v, SessionSourceType::User { task_id: None }));
    }

    #[test]
    fn deserialize_legacy_ambient_agent_bare() {
        let v: SessionSourceType = serde_json::from_str("\"AmbientAgent\"").unwrap();
        assert!(matches!(
            v,
            SessionSourceType::AmbientAgent { task_id: None }
        ));
    }

    #[test]
    fn deserialize_new_user_with_task_id() {
        let v: SessionSourceType = serde_json::from_str(r#"{"User":{"task_id":"abc"}}"#).unwrap();
        match v {
            SessionSourceType::User {
                task_id: Some(ref s),
            } if s == "abc" => {}
            other => panic!("expected User {{ task_id: Some(\"abc\") }}, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_new_user_with_null_task_id() {
        let v: SessionSourceType = serde_json::from_str(r#"{"User":{"task_id":null}}"#).unwrap();
        assert!(matches!(v, SessionSourceType::User { task_id: None }));
    }

    #[test]
    fn deserialize_new_user_without_task_id_field() {
        let v: SessionSourceType = serde_json::from_str(r#"{"User":{}}"#).unwrap();
        assert!(matches!(v, SessionSourceType::User { task_id: None }));
    }

    #[test]
    fn deserialize_new_ambient_agent_with_task_id() {
        let v: SessionSourceType =
            serde_json::from_str(r#"{"AmbientAgent":{"task_id":"xyz"}}"#).unwrap();
        match v {
            SessionSourceType::AmbientAgent {
                task_id: Some(ref s),
            } if s == "xyz" => {}
            other => panic!("expected AmbientAgent {{ task_id: Some(\"xyz\") }}, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_new_ambient_agent_with_null_task_id() {
        // Guards Redis rows written before Serialize emitted bare unit-variants.
        let v: SessionSourceType =
            serde_json::from_str(r#"{"AmbientAgent":{"task_id":null}}"#).unwrap();
        assert!(matches!(
            v,
            SessionSourceType::AmbientAgent { task_id: None }
        ));
    }

    #[test]
    fn deserialize_new_ambient_agent_without_task_id_field() {
        let v: SessionSourceType = serde_json::from_str(r#"{"AmbientAgent":{}}"#).unwrap();
        assert!(matches!(
            v,
            SessionSourceType::AmbientAgent { task_id: None }
        ));
    }

    // --- Serialization ---

    #[test]
    fn serialize_user_without_task_id_emits_bare_form() {
        let v = SessionSourceType::User { task_id: None };
        let json = serde_json::to_string(&v).unwrap();
        // Legacy bare form so older readers can still parse it.
        assert_eq!(json, "\"User\"");
    }

    #[test]
    fn serialize_user_with_task_id_emits_struct_form() {
        let v = SessionSourceType::User {
            task_id: Some("abc".to_string()),
        };
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#"{"User":{"task_id":"abc"}}"#);
    }

    #[test]
    fn serialize_ambient_agent_without_task_id_emits_bare_form() {
        let v = SessionSourceType::AmbientAgent { task_id: None };
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"AmbientAgent\"");
    }

    #[test]
    fn serialize_ambient_agent_with_task_id_emits_struct_form() {
        let v = SessionSourceType::AmbientAgent {
            task_id: Some("xyz".to_string()),
        };
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#"{"AmbientAgent":{"task_id":"xyz"}}"#);
    }

    // --- Roundtrip ---

    #[test]
    fn roundtrip_user_with_task_id() {
        let v = SessionSourceType::User {
            task_id: Some("abc".to_string()),
        };
        let json = serde_json::to_string(&v).unwrap();
        let parsed: SessionSourceType = serde_json::from_str(&json).unwrap();
        match parsed {
            SessionSourceType::User {
                task_id: Some(ref s),
            } if s == "abc" => {}
            other => panic!("roundtrip altered value: {other:?}"),
        }
    }

    #[test]
    fn roundtrip_ambient_agent_with_task_id() {
        let v = SessionSourceType::AmbientAgent {
            task_id: Some("xyz".to_string()),
        };
        let json = serde_json::to_string(&v).unwrap();
        let parsed: SessionSourceType = serde_json::from_str(&json).unwrap();
        match parsed {
            SessionSourceType::AmbientAgent {
                task_id: Some(ref s),
            } if s == "xyz" => {}
            other => panic!("roundtrip altered value: {other:?}"),
        }
    }

    // --- Helpers ---

    #[test]
    fn orchestrator_task_id_returns_user_task_id() {
        let v = SessionSourceType::User {
            task_id: Some("abc".to_string()),
        };
        assert_eq!(v.orchestrator_task_id(), Some("abc"));
    }

    #[test]
    fn orchestrator_task_id_returns_ambient_agent_task_id() {
        let v = SessionSourceType::AmbientAgent {
            task_id: Some("xyz".to_string()),
        };
        assert_eq!(v.orchestrator_task_id(), Some("xyz"));
    }

    #[test]
    fn orchestrator_task_id_none_when_missing() {
        assert_eq!(
            SessionSourceType::User { task_id: None }.orchestrator_task_id(),
            None
        );
        assert_eq!(
            SessionSourceType::AmbientAgent { task_id: None }.orchestrator_task_id(),
            None
        );
    }

    #[test]
    fn from_user_maps_to_legacy_user_regardless_of_task_id() {
        let no_task = SessionSourceType::User { task_id: None };
        assert!(matches!(
            LegacySessionSourceType::from(&no_task),
            LegacySessionSourceType::User
        ));

        let with_task = SessionSourceType::User {
            task_id: Some("abc".to_string()),
        };
        assert!(matches!(
            LegacySessionSourceType::from(&with_task),
            LegacySessionSourceType::User
        ));
    }

    #[test]
    fn from_ambient_agent_maps_to_legacy_ambient_agent_regardless_of_task_id() {
        let no_task = SessionSourceType::AmbientAgent { task_id: None };
        assert!(matches!(
            LegacySessionSourceType::from(&no_task),
            LegacySessionSourceType::AmbientAgent
        ));

        let with_task = SessionSourceType::AmbientAgent {
            task_id: Some("xyz".to_string()),
        };
        assert!(matches!(
            LegacySessionSourceType::from(&with_task),
            LegacySessionSourceType::AmbientAgent
        ));
    }

    #[test]
    fn default_is_user_without_task_id() {
        let v = SessionSourceType::default();
        assert!(matches!(v, SessionSourceType::User { task_id: None }));
    }
}
