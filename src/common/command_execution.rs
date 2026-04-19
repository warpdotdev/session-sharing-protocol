use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An opaque id to track command execution requests.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandExecutionRequestId(String);
impl CommandExecutionRequestId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

/// A set of reasons for which a command execution request might fail.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CommandExecutionFailureReason {
    /// The viewer does not have sufficient permissions to run commands.
    InsufficientPermissions,

    /// The buffer for which the command execution was requested is old.
    /// Specifically, there is either a newer buffer or a command
    /// is in-progress for the given buffer.
    StaleBuffer,
}
