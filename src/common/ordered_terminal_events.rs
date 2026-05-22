use super::{BlockId, ParticipantId};
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
    /// initial LLM turn will follow (e.g. empty-prompt local-to-cloud handoff with
    /// `--skip-initial-turn`). The viewer uses this to tear down the Cloud Mode Setup V2
    /// "Running setup commands…" chip and clear `BlockList::is_executing_oz_environment_startup_commands`
    /// without needing to wait for the first `AppendedExchange`.
    ///
    /// There is no matching `AmbientSetupPhaseStarted` variant: setup-phase start is already
    /// implicit in the existing `SetupCommandState` transitions on the viewer (the default
    /// `running_group_id` is `Some(initial)` once scrollback arrives), so an explicit start
    /// marker would be redundant.
    AmbientSetupPhaseEnded,
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
            Self::AmbientSetupPhaseEnded => f.write_str("AmbientSetupPhaseEnded"),
        }
    }
}

impl OrderedTerminalEventType {
    pub fn num_bytes(&self) -> Byte {
        match &self {
            OrderedTerminalEventType::PtyBytesRead { bytes } => bytes.len().into(),
            OrderedTerminalEventType::AgentResponseEvent { response_event, .. } => {
                response_event.len().into()
            }
            OrderedTerminalEventType::CommandExecutionStarted { .. }
            | OrderedTerminalEventType::CommandExecutionFinished { .. }
            | OrderedTerminalEventType::AgentConversationReplayStarted
            | OrderedTerminalEventType::AgentConversationReplayEnded
            | OrderedTerminalEventType::AmbientSetupPhaseEnded
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
    pub fn num_bytes(&self) -> Byte {
        self.event_type.num_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambient_setup_phase_ended_serializes_as_unit_variant() {
        let event_type = OrderedTerminalEventType::AmbientSetupPhaseEnded;
        let json = serde_json::to_string(&event_type).expect("serialize");
        assert_eq!(json, "\"AmbientSetupPhaseEnded\"");
    }

    #[test]
    fn ambient_setup_phase_ended_round_trips() {
        let event = OrderedTerminalEvent {
            event_no: 42,
            event_type: OrderedTerminalEventType::AmbientSetupPhaseEnded,
        };
        let json = serde_json::to_string(&event).expect("serialize");
        let parsed: OrderedTerminalEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.event_no, 42);
        assert!(matches!(
            parsed.event_type,
            OrderedTerminalEventType::AmbientSetupPhaseEnded
        ));
        assert_eq!(parsed.num_bytes(), Byte::from_u64(0));
    }
}
