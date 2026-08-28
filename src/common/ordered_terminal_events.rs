use super::{BlockId, ParticipantId, SemanticCursor};
use byte_unit::Byte;
use serde::{Deserialize, Serialize};

/// AI metadata for correlating terminal blocks with agent commands.
/// This allows viewers in shared sessions to associate terminal command blocks
/// with the agent tool calls that triggered them.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AICommandMetadata {
    /// The tool call ID from the Multi-Agent API protocol.
    /// Corresponds to action_id on the sharer side.
    pub tool_call_id: String,

    /// Whether this command is being monitored by an agent as a long-running command.
    #[serde(default)]
    pub is_agent_monitored: bool,
}

/// Types of terminal events that need to be ordered against each other.
#[derive(Clone, Deserialize, Serialize)]
pub enum OrderedTerminalEventType {
    /// Bytes read off the sharer's pty (session contents).
    PtyBytesRead {
        bytes: Vec<u8>,
    },
    /// A command is beginning to execute.
    CommandExecutionStarted {
        /// The ID of the participant who ran the command.
        participant_id: ParticipantId,
        /// AI metadata if this command was executed by an agent.
        #[serde(default)]
        ai_metadata: Option<AICommandMetadata>,
    },
    CommandExecutionFinished {
        next_block_id: BlockId,
    },
    /// The sharer's terminal was resized.
    Resize {
        window_size: WindowSize,
    },
    /// The sharer received an AI agent response event. Response events include all information needed to reconstruct a conversation, including:
    // * The start and end of individual requests
    // * Incremental agent output
    // * Echoed user messages and tool call results
    /// See https://github.com/warpdotdev/warp-proto-apis/blob/6310871f081b5f44b2d4e3e5d8fdfa3008b750b0/apis/multi_agent/v1/response.proto#L16-L17
    AgentResponseEvent {
        /// The ID of the participant who sent the query to initiate this agent response.
        response_initiator: Option<ParticipantId>,
        /// The base64-encoded MAA ResponseEvent protocol buffer message.
        response_event: String,
        /// For forked conversations, this is the original conversation token that the
        /// conversation was forked from. Viewers use this to link the new server-assigned
        /// conversation token to an existing conversation created during historical replay.
        #[serde(default)]
        forked_from_conversation_token: Option<String>,
    },
    /// Marks the start of historical agent conversation replay.
    /// Viewers should use this to suppress live-conversation specific actions until replay ends
    /// (e.g. the insertion of the ambient agent conversation tombstone).
    AgentConversationReplayStarted,
    /// Marks the end of historical agent conversation replay.
    AgentConversationReplayEnded,
    /// Emitted by the sandboxed Oz AgentDriver when the cloud-mode setup phase is complete but no
    /// initial LLM turn will follow (e.g. empty-prompt local-to-cloud handoff with `--skip-initial-turn`).
    CloudModeSetupPhaseEnded,
    /// A serialized ConversationMutation protobuf.
    ///
    /// Servers must only send this variant to an endpoint that positively
    /// negotiated SemanticConversationOnly and supports_semantic_conversation.
    SemanticConversationMutation {
        cursor: SemanticCursor,
        mutation: Vec<u8>,
    },
}

/// Represents the size of a PTY. Mimics the winsize struct that
/// can be queried via [ioctl](https://man7.org/linux/man-pages/man2/ioctl_tty.2.html).
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct WindowSize {
    pub num_rows: usize,
    pub num_cols: usize,
}

/// Override the Debug impl to avoid accidentally leaking sensitive
/// data in logs.
impl std::fmt::Debug for OrderedTerminalEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PtyBytesRead { .. } => f.write_str("PtyBytesRead"),
            Self::CommandExecutionStarted { .. } => f.write_str("CommandExecutionStarted"),
            Self::CommandExecutionFinished { .. } => f.write_str("CommandExecutionFinished"),
            Self::Resize { .. } => f.write_str("Resize"),
            Self::AgentResponseEvent { .. } => f.write_str("AgentResponseEvent"),
            Self::AgentConversationReplayStarted => f.write_str("AgentConversationReplayStarted"),
            Self::AgentConversationReplayEnded => f.write_str("AgentConversationReplayEnded"),
            Self::CloudModeSetupPhaseEnded => f.write_str("CloudModeSetupPhaseEnded"),
            Self::SemanticConversationMutation { .. } => {
                f.write_str("SemanticConversationMutation")
            }
        }
    }
}

impl OrderedTerminalEventType {
    /// Whether this variant is unknown to legacy peers and therefore requires
    /// positive semantic negotiation before it is sent.
    pub fn requires_semantic_support(&self) -> bool {
        matches!(self, Self::SemanticConversationMutation { .. })
    }
    pub fn num_bytes(&self) -> Byte {
        match &self {
            OrderedTerminalEventType::PtyBytesRead { bytes } => bytes.len().into(),
            OrderedTerminalEventType::AgentResponseEvent { response_event, .. } => {
                response_event.len().into()
            }
            OrderedTerminalEventType::SemanticConversationMutation { mutation, .. } => {
                mutation.len().into()
            }
            OrderedTerminalEventType::CommandExecutionStarted { .. }
            | OrderedTerminalEventType::CommandExecutionFinished { .. }
            | OrderedTerminalEventType::AgentConversationReplayStarted
            | OrderedTerminalEventType::AgentConversationReplayEnded
            | OrderedTerminalEventType::CloudModeSetupPhaseEnded
            | OrderedTerminalEventType::Resize { .. } => Byte::from_u64(0),
        }
    }
}

/// Any terminal event where strict ordering against other terminal events is important.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrderedTerminalEvent {
    pub event_no: usize,
    pub event_type: OrderedTerminalEventType,
}

impl OrderedTerminalEvent {
    pub fn requires_semantic_support(&self) -> bool {
        self.event_type.requires_semantic_support()
    }
    pub fn num_bytes(&self) -> Byte {
        self.event_type.num_bytes()
    }
}
