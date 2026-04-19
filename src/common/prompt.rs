use serde::{Deserialize, Serialize};

// The prompt for the active block.
#[derive(Clone, Default, Deserialize, PartialEq, Eq, Serialize)]
pub enum ActivePrompt {
    /// Using the PS1 prompt, which is included in forwarded pty bytes.
    #[default]
    PS1,
    /// JSON serialization of PromptSnapshot
    WarpPrompt(String),
}

/// Override the Debug impl to avoid accidentally leaking sensitive
/// data in logs.
impl std::fmt::Debug for ActivePrompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PS1 { .. } => f.write_str("ActivePrompt::PS1"),
            Self::WarpPrompt(..) => f.write_str("ActivePrompt::WarpPrompt"),
        }
    }
}

// An update to the active block's prompt.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActivePromptUpdate {
    pub active_prompt: ActivePrompt,
    /// The event_no of the last OrderedTerminalEvent shared.
    pub last_event_no: usize,
}
