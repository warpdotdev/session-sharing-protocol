use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::ServerConversationToken;

/// A unique id to track control action requests.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlActionRequestId(String);

impl ControlActionRequestId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Returns the underlying opaque id string.
    pub fn id(&self) -> &str {
        &self.0
    }
}

/// Higher-level control messages that don't correspond 1:1 to terminal actions/inputs
/// (or imply a warp-specific action outside of their normal terminal use).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ControlAction {
    /// Request that a shared-session AI conversation be cancelled by the sharer.
    CancelConversation {
        server_conversation_token: ServerConversationToken,
    },
}

/// Reasons a control action request from a viewer might fail.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ControlActionFailureReason {
    /// The viewer does not have permission to perform this control action.
    InsufficientPermissions,
    /// The session no longer exists.
    SessionNotFound,
    /// There is no sharer currently connected to handle the action.
    SharerUnavailable,
    /// Unexpected, something went wrong in the server.
    InternalServerError,
}
