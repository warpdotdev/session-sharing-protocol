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
    RoleRequestId, RoleRequestResponse, Selection, SelectionUpdate, ServerConversationToken,
    SessionId, SessionSecret, TelemetryContext, UniversalDeveloperInputContext,
    UniversalDeveloperInputContextUpdate, UserID, WindowSize, WriteToPtyFailureReason,
    WriteToPtyRequestId,
};

use super::common::Scrollback;
use byte_unit::Byte;
use serde::{Deserialize, Deserializer, Serialize};
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

#[derive(Clone, Debug, Serialize, Default)]
pub enum SessionSourceType {
    /// The session was started by a user directly.
    #[default]
    User,
    /// The session was started in the course of spinning up an ambient agent.
    AmbientAgent {
        #[serde(default)]
        task_id: Option<String>,
    },
}

/// Mirrors the legacy unit-variant form and the new struct-variant form so
/// the custom `Deserialize` impl can accept both shapes.
#[derive(Deserialize)]
#[serde(untagged)]
enum SessionSourceTypeWire {
    /// Legacy representation: bare `"User"` or `"AmbientAgent"`.
    Legacy(LegacySessionSourceType),
    /// New representation: externally tagged `AmbientAgent` with fields, e.g.
    /// `{ "AmbientAgent": { "task_id": "..." } }`.
    New {
        #[serde(rename = "AmbientAgent")]
        ambient_agent: AmbientAgentFields,
    },
}

#[derive(Deserialize)]
struct AmbientAgentFields {
    #[serde(default)]
    task_id: Option<String>,
}

impl From<SessionSourceTypeWire> for SessionSourceType {
    fn from(value: SessionSourceTypeWire) -> Self {
        match value {
            SessionSourceTypeWire::Legacy(LegacySessionSourceType::User) => SessionSourceType::User,
            SessionSourceTypeWire::Legacy(LegacySessionSourceType::AmbientAgent) => {
                SessionSourceType::AmbientAgent { task_id: None }
            }
            SessionSourceTypeWire::New {
                ambient_agent: AmbientAgentFields { task_id },
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

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum LegacySessionSourceType {
    #[default]
    User,
    AmbientAgent,
}

impl From<&SessionSourceType> for LegacySessionSourceType {
    fn from(value: &SessionSourceType) -> Self {
        match value {
            SessionSourceType::User => LegacySessionSourceType::User,
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

/// The reasons the sharer may request extended retention for a shared session.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub enum SessionRetentionReason {
    /// Environment setup failed. The session should remain available so users can inspect setup
    /// command output from the shared session link after the sandbox shuts down.
    #[default]
    SetupFailed,
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

    /// Optional orchestrator `task_id` carried alongside `source_type`.
    /// Set when the sharer wants downstream orchestration discovery to find
    /// this share's children regardless of variant kind. Sidecar so the
    /// `User` variant can stay a unit and old viewers ignore it.
    #[serde(default)]
    pub source_task_id: Option<String>,

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
// `Initialize(InitPayload)` is much larger than the other variants because
// `InitPayload` carries scrollback and feature-support data. Boxing it would
// be wire-compatible but churn every call site; suppress the lint instead.
#[allow(clippy::large_enum_variant)]
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

    /// The client sends this message to request that the server retain session data longer than
    /// normal after sharing ends.
    ExtendSessionRetention {
        #[serde(default)]
        reason: SessionRetentionReason,
    },

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
        /// Echoes the originating request's idempotency key when it was a `purpose`-tagged
        /// bootstrap request (REMOTE-2661), so the server can persist the rejection under the
        /// same key a caller's retry will look up. `None` for an ordinary rejection.
        #[serde(default)]
        idempotency_key: Option<String>,
    },

    /// Reports the conversation the sharer created or reused for a `purpose`-tagged agent
    /// prompt request that carried no `server_conversation_token` (REMOTE-2661). Never sent
    /// for an ordinary agent prompt request (one with `purpose: None`), since the server
    /// already knows that conversation's token by other means. The server must persist this
    /// before the request may be treated as delivered: a lost acknowledgement here is what
    /// would otherwise let a retry start a second, independent conversation.
    AcknowledgeAgentPromptRequest {
        id: AgentPromptRequestId,
        participant_id: ParticipantId,
        server_conversation_token: ServerConversationToken,
        /// Echoes the originating request's idempotency key, which the server correlates
        /// against its own pending wait for this bootstrap's result.
        idempotency_key: String,
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
