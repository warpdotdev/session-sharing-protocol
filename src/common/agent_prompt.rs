use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::BlockId;

/// An opaque id to track agent prompt requests from viewers.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentPromptRequestId(String);

impl AgentPromptRequestId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for AgentPromptRequestId {
    fn default() -> Self {
        Self::new()
    }
}

/// A set of reasons for which an agent prompt request might fail.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentPromptFailureReason {
    /// The viewer does not have sufficient permissions to send agent prompts.
    InsufficientPermissions,

    /// The conversation ID provided is invalid or doesn't exist.
    InvalidConversation,

    // There is a long running command that is already in progress.
    CommandInProgress,
}

/// Represents an AI agent attachment that can be sent with a prompt.
/// This is a simplified version for the protocol - the sharer will reconstruct
/// the full attachment from the block ID.
///
/// TODO: Add support for image attachments. Images are currently handled as
/// AIAgentContext::Image (contextual info) rather than AIAgentAttachment in the client,
/// so we need to decide whether to treat viewer-attached images as context or attachments
/// before adding them to the protocol.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentAttachment {
    /// A reference to a terminal block by ID.
    /// The sharer will resolve this to the actual block content.
    BlockReference { block_id: BlockId },

    /// Plain text attachment (e.g., clipboard content).
    PlainText { content: String },

    /// A reference to a file that has been uploaded to GCS.
    /// The host fetches download URLs directly from warp-server using the attachment ID.
    FileReference {
        attachment_id: String,
        file_name: String,
    },
}

/// An optional server conversation token for continuing an existing agent conversation.
/// The id provided here is 1:1 with the ServerConversationToken used on the client/server (not AIAgentConversationId).
/// If no token is provided, a new conversation will be started.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub struct ServerConversationToken(Uuid);

impl ServerConversationToken {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ServerConversationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ServerConversationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ServerConversationToken {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// The data for an agent prompt request from a viewer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentPromptRequest {
    /// Unique identifier for this request.
    pub id: AgentPromptRequestId,

    /// The server conversation token to continue. If None, start a new conversation.
    /// This is the server_conversation_token that links viewer and sharer conversations.
    pub server_conversation_token: Option<ServerConversationToken>,

    /// The user's prompt/query.
    pub prompt: String,

    /// Optional attachments (blocks, files, etc.) referenced in the prompt.
    #[serde(default)]
    pub attachments: Vec<AgentAttachment>,

    /// Idempotency key for a bootstrap request: set, together with a
    /// `server_conversation_token` of `None`, when the request comes from an authenticated
    /// server-side injection that must create or reuse exactly one conversation (REMOTE-2661).
    /// `None` for an ordinary agent prompt request from a live viewer.
    ///
    /// The key exists so the injector can correlate a retry with its original attempt.
    /// Deduplication belongs to the session-sharing service, which claims the key before
    /// injecting, so a retry of a key it has already answered does not reach the sharer. The
    /// sharer is not required to keep per-key state.
    #[serde(default)]
    pub idempotency_key: Option<String>,
}
