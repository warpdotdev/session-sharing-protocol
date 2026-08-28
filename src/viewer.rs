//! The message types that are communicated between the
//! server and a viewer client.
//!
//! When a client wants to join a shared session, the client
//! will make a request against /sessions/join/:uuid. The client
//! must then send an [`Initialize`] message with the relevant data
//! to join the shared session. If successful, the server
//! will acknowledge the joining of the shared
//! session via the [`JoinedSuccessfully`] message.
//!
//! To reconnect to a shared session, the viewer can use the same
//! join endpoint, setting the init payload appropriately.
//! The server will acknowledge the rejoining via the
//! [`RejoinedSuccessfully`] message.

use crate::{
    common::{
        ActivePrompt, ActivePromptUpdate, AgentAttachment, AgentPromptFailureReason,
        AgentPromptRequest, AgentPromptRequestId, BlockId, BufferId, CommandExecutionFailureReason,
        CommandExecutionRequestId, ControlAction, ControlActionFailureReason, ExecutionIdentity,
        FeatureSupport, InputOperationId, InputReplicaId, InputUpdate, InputUpdateFailureReason,
        LinkAccessLevelUpdateResponse, NegotiatedSessionContent, OrderedTerminalEvent,
        ParticipantId, ParticipantList, ParticipantPresenceUpdate, Role, RoleRequestId,
        RoleRequestResponse, Scrollback, SelectionUpdate, SemanticCursor, SemanticResyncReason,
        SessionContentMode, TeamAccessLevelUpdateResponse, TeamAclData, TelemetryContext,
        UniversalDeveloperInputContext, UniversalDeveloperInputContextUpdate, UserID, WindowSize,
        WriteToPtyFailureReason, WriteToPtyRequestId,
    },
    sharer::{self, LegacySessionSourceType, SessionSourceType},
};
use byte_unit::Byte;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
/// Sent by sharer client or server
/// when the shared session has been ended.
pub enum SessionEndedReason {
    /// Unexpected, means something went wrong in the server.
    InternalServerError,
    /// The session was ended gracefully.
    EndedBySharer,
    /// The sharer was idle for too long.
    InactivityLimitReached,

    /// DEPRECATED
    ExceededSizeLimit,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FailedToJoinReason {
    /// Unexpected, means something went wrong in the server.
    Invalid,
    SessionNotFound,
    WrongPassword,
    InternalServerError,
    MaxNumberOfParticipantsReached,
    SessionNotAccessible,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, Copy)]
pub enum RoleUpdatedReason {
    #[default]
    UpdatedBySharer,
    InactivityLimitReached,
}

impl From<sharer::RoleUpdateReason> for RoleUpdatedReason {
    fn from(value: sharer::RoleUpdateReason) -> Self {
        match value {
            sharer::RoleUpdateReason::UpdatedBySharer => RoleUpdatedReason::UpdatedBySharer,
            sharer::RoleUpdateReason::InactivityLimitReached => {
                RoleUpdatedReason::InactivityLimitReached
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum ViewerRemovedReason {
    LostAccess,
}

/// The initial state that the viewer must supply when joining or rejoining
/// a shared session.
#[derive(Debug, Deserialize, Serialize)]
pub struct InitPayload {
    /// The ID previously assigned to this viewer when joining.
    /// Should be specified if the viewer is rejoining.
    pub viewer_id: Option<ParticipantId>,
    pub user_id: UserID,
    /// If the viewer is reconnecting, they should specify the last event no received.
    /// The server will only send events after this event no.
    pub last_received_event_no: Option<usize>,

    /// The ID of the last block the viewer has seen.
    /// Should only be specified when re-joining.
    pub latest_block_id: Option<BlockId>,

    pub telemetry_context: Option<TelemetryContext>,

    /// Client feature support declaration.
    #[serde(default)]
    pub feature_support: FeatureSupport,

    /// Requested session content mode. Omitted on the legacy full-terminal wire.
    #[serde(default, skip_serializing_if = "SessionContentMode::is_full_terminal")]
    pub content_mode: SessionContentMode,

    /// Required when joining a semantic-only session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_schema_version: Option<u32>,

    /// Exact execution expected by a semantic bootstrap caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_identity: Option<ExecutionIdentity>,

    /// Last contiguous semantic mutation already applied by a rejoining viewer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_cursor: Option<SemanticCursor>,
}

/// The possible messages sent from server to client (viewer).
#[derive(Serialize, Deserialize, Clone)]
pub enum DownstreamMessage {
    /// The server sends this message when the session was successfully joined.
    /// TODO: add initial state to pass to viewer (e.g. size info)
    JoinedSuccessfully {
        scrollback: Box<Scrollback>,
        /// The sharer's active prompt.
        active_prompt: ActivePrompt,
        /// The latest event no of the session the viewer will be catching up to.
        /// If None, there are no events to catch up to.
        latest_event_no: Option<usize>,
        window_size: WindowSize,

        participant_list: Box<ParticipantList>,

        /// The ID assigned to this viewer
        viewer_id: ParticipantId,
        /// The Firebase UID assigned to this viewer.
        viewer_firebase_uid: String,

        /// The block ID of the first block after scrollback.
        /// The viewer can use this to identify buffer updates for
        /// the first block.
        init_block_id: BlockId,

        input_replica_id: InputReplicaId,

        /// The universal developer input context (model selection, etc.).
        #[serde(default)]
        universal_developer_input_context: Option<UniversalDeveloperInputContext>,

        /// The legacy source type for this shared session (i.e. user or ambient agent).
        #[serde(default)]
        #[deprecated(note = "please use `detailed_source_type` instead")]
        source_type: LegacySessionSourceType,

        /// The detailed source type for this shared session.
        #[serde(default)]
        detailed_source_type: SessionSourceType,

        /// Optional orchestrator `task_id` carried alongside the source
        /// type, mirroring `sharer::InitPayload::source_task_id`. Lets
        /// viewers find this share's orchestrator task without keying
        /// off the source-type variant kind.
        #[serde(default)]
        source_task_id: Option<String>,

        /// Present only when the server positively negotiated semantic content.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        negotiated_content: Option<NegotiatedSessionContent>,
    },

    /// The server sends this message when the session was successfully rejoined.
    RejoinedSuccessfully {
        participant_list: Box<ParticipantList>,
        /// Present only when the server positively negotiated semantic content.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        negotiated_content: Option<NegotiatedSessionContent>,
    },

    /// The semantic session can no longer continue from its current cursor.
    ///
    /// Servers must not send this variant to an endpoint that did not positively
    /// negotiate semantic conversation support.
    SemanticResyncRequired { reason: SemanticResyncReason },

    /// Sent when the viewer fails to join the shared session.
    /// The client should not expect any more messages after this.
    FailedToJoin { reason: FailedToJoinReason },

    /// Sent when the shared session has been ended.
    /// The client should not expect any more messages after this.
    SessionEnded { reason: SessionEndedReason },

    /// Update to the sharer's active prompt.
    ActivePromptUpdated(ActivePromptUpdate),

    /// Update to the universal developer input context (model selection, etc.) from sharer or editor viewers.
    UniversalDeveloperInputContextUpdated(UniversalDeveloperInputContextUpdate),

    /// Sent when there is any ordered terminal event.
    /// These messages are only sent _after_ [`DownstreamMessage::JoinedSuccessfully`].
    OrderedTerminalEvent(OrderedTerminalEvent),

    /// Sent when the list of participants in the shared session changes.
    ParticipantListUpdated(ParticipantList),

    /// Sent when a participant's presence changes.
    ParticipantPresenceUpdated(ParticipantPresenceUpdate),

    /// The server has acknowledged the role request and sent it to the sharer.
    /// There can only be at most one role request in flight per participant.
    RoleRequestInFlight(RoleRequestId),

    /// The viewer's role request was responded to.
    RoleRequestResponse(RoleRequestResponse),

    /// A participant's (identified by `participant_id`) role was updated.
    ParticipantRoleChanged {
        participant_id: ParticipantId,
        reason: RoleUpdatedReason,
        role: Role,
    },

    /// The input was updated by a participant.
    /// When we receive our own update, we can treat it as an ack.
    InputUpdated(InputUpdate),

    /// An input operation was rejected and should be undone.
    InputUpdateRejected {
        id: InputOperationId,
        reason: InputUpdateFailureReason,
    },

    /// The server has acknowledged the command execution request and sent it to the sharer.
    /// There can only be at most one command execution request in flight per participant.
    CommandExecutionRequestInFlight(CommandExecutionRequestId),

    /// The viewer's command execution request failed.
    /// Note: there is no "success" response; that is implicitly handled
    /// by the fact that the command is executed.
    CommandExecutionRequestFailed {
        id: CommandExecutionRequestId,
        reason: CommandExecutionFailureReason,
    },

    /// The viewer's write to pty request failed.
    WriteToPtyRequestFailed { reason: WriteToPtyFailureReason },

    /// The server has acknowledged the agent prompt request and sent it to the sharer.
    AgentPromptRequestInFlight(AgentPromptRequestId),

    /// The viewer's agent prompt request failed.
    /// Note: there is no "success" response; that is implicitly handled
    /// by the fact that agent response events start streaming.
    AgentPromptRequestFailed { reason: AgentPromptFailureReason },

    /// The viewer's control action request failed.
    ControlActionRequestFailed { reason: ControlActionFailureReason },

    /// The viewer was removed from the session by the sharer.
    ViewerRemoved { reason: ViewerRemovedReason },

    /// Deprecated: superseded by [`DownstreamMessage::LinkAccessLevelUpdateResponse`].
    /// Kept temporarily for backward compatibility with older clients. Remove once
    /// all clients handle `LinkAccessLevelUpdateResponse`.
    LinkAccessLevelUpdated { role: Option<Role> },

    /// Deprecated: superseded by [`DownstreamMessage::TeamAccessLevelUpdateResponse`].
    /// Kept temporarily for backward compatibility with older clients. Remove once
    /// all clients handle `TeamAccessLevelUpdateResponse`.
    TeamAccessLevelUpdated {
        /// The UID of the updated team.
        team_uid: String,
        /// The ACL of the updated team. None if team has no ACL.
        team_acl: Option<TeamAclData>,
    },

    /// The viewer's link access level update request was responded to.
    LinkAccessLevelUpdateResponse(crate::common::LinkAccessLevelUpdateResponse),

    /// The request to add guests was responded to.
    AddGuestsResponse(crate::common::AddGuestsResponse),

    /// The request to remove a guest was responded to.
    RemoveGuestResponse(crate::common::RemoveGuestResponse),

    /// The request to update a pending user role was responded to.
    UpdatePendingUserRoleResponse(crate::common::UpdatePendingUserRoleResponse),

    /// The viewer's team access level update request was responded to.
    TeamAccessLevelUpdateResponse(crate::common::TeamAccessLevelUpdateResponse),

    /// A response to a [`UpstreamMessage::Ping`].
    /// Used to demonstrate that the server is still alive.
    Pong { data: Vec<u8> },
}

impl DownstreamMessage {
    /// Whether sending this message requires positive semantic negotiation.
    pub fn requires_semantic_support(&self) -> bool {
        match self {
            Self::SemanticResyncRequired { .. } => true,
            Self::OrderedTerminalEvent(event) => event.requires_semantic_support(),
            _ => false,
        }
    }
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Downgrades all `Role::Full` fields to `Role::Executor`.
    /// Used for backward compatibility with clients that don't support the Full role.
    #[allow(deprecated)]
    pub fn downgrade_full_roles(&mut self) {
        match self {
            Self::JoinedSuccessfully {
                participant_list, ..
            } => participant_list.downgrade_full_roles(),
            Self::RejoinedSuccessfully {
                participant_list, ..
            } => participant_list.downgrade_full_roles(),
            Self::ParticipantListUpdated(list) => list.downgrade_full_roles(),
            Self::ParticipantRoleChanged { role, .. } => role.downgrade_full(),
            Self::RoleRequestResponse(RoleRequestResponse::Approved { new_role }) => {
                new_role.downgrade_full()
            }
            Self::LinkAccessLevelUpdated { role: Some(role) } => {
                role.downgrade_full();
            }
            Self::TeamAccessLevelUpdated {
                team_acl: Some(team_acl),
                ..
            } => {
                team_acl.acl.downgrade_full();
            }
            Self::LinkAccessLevelUpdateResponse(LinkAccessLevelUpdateResponse::Ok {
                role: Some(role),
            }) => {
                role.downgrade_full();
            }
            Self::TeamAccessLevelUpdateResponse(TeamAccessLevelUpdateResponse::Success {
                team_acl: Some(team_acl),
                ..
            }) => {
                team_acl.acl.downgrade_full();
            }
            _ => {}
        }
    }
}

/// The possible messages sent from client (viewer) to server.
///
/// `Initialize` intentionally remains unboxed to preserve the existing public
/// construction and match API. Serde's JSON wire representation would not
/// benefit from boxing the payload.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Serialize, Deserialize)]
pub enum UpstreamMessage {
    /// The client sends this message to join the shared session.
    Initialize(InitPayload),

    /// A heartbeat message to demonstrate that the
    /// client is still alive.
    Ping { data: Vec<u8> },

    /// Sent when the viewer changes what they have selected.
    UpdateSelection(SelectionUpdate),

    /// The viewer is requesting a new role.
    RequestRole(Role),

    /// The viewer no longer wants to change roles.
    CancelRoleRequest(RoleRequestId),

    /// The viewer updated their input.
    /// This is an optimistic update and thus was already applied on the viewer's client.
    UpdateInput(InputUpdate),

    /// The viewer is requesting the sharer to execute the provided command
    /// in the given buffer.
    ExecuteCommand {
        buffer_id: BufferId,
        command: String,
    },

    /// The viewer is requesting to write to the pty,
    /// specifically to a long running command.
    WriteToPty {
        request_id: WriteToPtyRequestId,
        bytes: Vec<u8>,
    },

    /// The viewer is requesting to send an agent prompt.
    /// If there's an existing in-flight request for the same conversation,
    /// it will be cancelled and replaced with this new request.
    SendAgentPrompt(AgentPromptRequest),

    /// The viewer (with Editor role) is updating the universal developer input context.
    UpdateUniversalDeveloperInputContext(UniversalDeveloperInputContextUpdate),

    /// The viewer is requesting a one-off control action to be applied to the shared session.
    SendControlAction(ControlAction),

    /// The viewer has reauthenticated.
    Reauthenticated { user_id: UserID },

    /// The viewer updated the session's link permissions.
    UpdateLinkAccessLevel { role: Option<Role> },

    /// The viewer updated the session's team permissions.
    UpdateTeamAccessLevel {
        team_uid: String,
        role: Option<Role>,
    },

    /// The viewer added users as session guests by email.
    AddGuests { emails: Vec<String>, role: Role },

    /// The viewer removed a user as a session guest.
    RemoveGuest { user_uid: String },

    /// The viewer removed a pending user as a session guest.
    RemovePendingGuest { email: String },

    /// The viewer changed a user's role.
    UpdateUserRole { user_uid: String, role: Role },

    /// The viewer changed a pending user's role.
    UpdatePendingUserRole { email: String, role: Role },

    /// The viewer is reporting its terminal size to the sharer.
    /// Used for remote-control sessions where the viewer's viewport should drive the PTY size.
    ReportTerminalSize { window_size: WindowSize },
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
            UpstreamMessage::UpdateInput(input_update) => input_update.num_bytes(),
            UpstreamMessage::ExecuteCommand { command, .. } => command.len().into(),
            UpstreamMessage::WriteToPty { bytes, .. } => bytes.len().into(),
            UpstreamMessage::SendAgentPrompt(request) => {
                // Count prompt length + attachments
                let prompt_bytes: Byte = request.prompt.len().into();
                let attachments_bytes: Byte = request
                    .attachments
                    .iter()
                    .map(|att| match att {
                        AgentAttachment::PlainText { content } => content.len(),
                        // Block's are already included in the shared session thus far,
                        // so we do not have to count them again here.
                        AgentAttachment::BlockReference { .. } => 0,
                        // FileReference is just IDs — the actual data is in GCS.
                        AgentAttachment::FileReference { .. } => 0,
                    })
                    .sum::<usize>()
                    .into();
                prompt_bytes
                    .add(attachments_bytes)
                    .unwrap_or(u64::MAX.into())
            }
            _ => Byte::from_u64(0),
        }
    }
}
