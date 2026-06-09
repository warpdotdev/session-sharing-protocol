use serde::{Deserialize, Serialize};

use crate::common::{BlockId, ServerConversationToken};

/// The active base model selection for agent mode.
/// This represents the UI state of which model is selected in the model picker chip.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SelectedAgentModel(String);

impl SelectedAgentModel {
    pub fn new(model_id: impl Into<String>) -> Self {
        Self(model_id.into())
    }

    pub fn model_id(&self) -> &str {
        &self.0
    }
}

/// The selected conversation for agent mode.
/// When agent view is enabled, this represents the conversation that is currently expanded.
/// When agent view is disabled, this represents the conversation the next query will follow up in.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum SelectedConversation {
    /// An existing conversation identified by a server token
    ExistingConversation(ServerConversationToken),
    /// The next query will start a new conversation
    /// (when agent view is enabled, this looks like an empty expanded view).
    #[default]
    NewConversation,
    /// No conversation selected
    /// (when agent view is enabled, this means that no agent view is expanded).
    NoConversation,
}

impl SelectedConversation {
    pub fn new(server_token: Option<ServerConversationToken>) -> Self {
        match server_token {
            Some(token) => Self::ExistingConversation(token),
            None => Self::NewConversation,
        }
    }

    pub fn server_token(&self) -> Option<&ServerConversationToken> {
        match self {
            Self::ExistingConversation(token) => Some(token),
            Self::NewConversation | Self::NoConversation => None,
        }
    }

    pub fn is_new_conversation(&self) -> bool {
        matches!(self, Self::NewConversation)
    }

    pub fn is_no_conversation(&self) -> bool {
        matches!(self, Self::NoConversation)
    }
}

/// The input type for the universal developer input.
#[derive(Clone, Default, Deserialize, Serialize, PartialEq, Eq, Debug, Copy)]
pub enum InputType {
    /// The user input is a shell command.
    #[default]
    Shell,

    /// The user input is a natural language query to AI.
    AI,
}

/// The input mode for the universal developer input
/// (i.e. the input type and whether the input is locked in said type)
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct InputMode {
    pub input_type: InputType,
    pub is_locked: bool,
}

impl InputMode {
    pub fn new(input_type: InputType, is_locked: bool) -> Self {
        Self {
            input_type,
            is_locked,
        }
    }
}

/// Whether a CLI agent (e.g. Claude Code, Gemini CLI) is active in the
/// terminal. Synced during shared sessions so viewers see the agent footer.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum CLIAgentSessionState {
    /// A CLI agent is running.
    Active {
        /// Serialized `CLIAgent` enum value (e.g. "Claude", "Gemini", "Codex").
        cli_agent: String,
        /// Whether the CLI agent rich input composer is open.
        is_rich_input_open: bool,
    },
    /// No CLI agent is running (or the previous one ended).
    #[default]
    Inactive,
}

/// How the agent is interacting with the current long running command (if at all).
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum LongRunningCommandAgentInteractionState {
    /// The agent is not interacting with any long running command.
    NotInteracting,
    /// The user started a long running command and tagged the agent into it.
    TaggedIn,
    /// The agent started and is controlling a long running command.
    InControl,
}

/// How the agent is interacting with a specific long running command block.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LongRunningCommandAgentInteraction {
    pub block_id: BlockId,
    pub state: LongRunningCommandAgentInteractionState,
}

/// The combined state container for universal developer input context.
/// This includes model selection, input mode, and selected conversation.
#[derive(Clone, Debug, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct UniversalDeveloperInputContext {
    /// Which agent model is selected as the primary model.
    pub selected_model: Option<SelectedAgentModel>,

    /// The input mode for the universal developer input.
    pub input_mode: Option<InputMode>,

    /// The selected conversation (identified by server token) for the next agent query.
    pub selected_conversation: Option<SelectedConversation>,

    /// How the agent is interacting with the current long running command (if at all).
    /// Deprecated in favor of long_running_command_agent_interaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_running_command_agent_interaction_state:
        Option<LongRunningCommandAgentInteractionState>,

    /// How the agent is interacting with a specific long running command block, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_running_command_agent_interaction: Option<LongRunningCommandAgentInteraction>,

    /// Whether auto-approve is enabled for agent actions.
    pub auto_approve_agent_actions: Option<bool>,

    /// Whether a CLI agent is active in this terminal.
    #[serde(default)]
    pub cli_agent_session: CLIAgentSessionState,
}

/// Update message for universal developer input context - only contains fields that changed.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct UniversalDeveloperInputContextUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_model: Option<SelectedAgentModel>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_mode: Option<InputMode>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_conversation: Option<SelectedConversation>,

    /// How the agent is interacting with the current long running command (if at all).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_running_command_agent_interaction_state:
        Option<LongRunningCommandAgentInteractionState>,

    /// How the agent is interacting with a specific long running command block, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_running_command_agent_interaction: Option<LongRunningCommandAgentInteraction>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_approve_agent_actions: Option<bool>,

    /// Whether a CLI agent is active. `None` = no change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli_agent_session: Option<CLIAgentSessionState>,
}

impl UniversalDeveloperInputContextUpdate {
    /// Returns true if this update would actually change the given cached context.
    pub fn changes_cached_context(&self, cached: &UniversalDeveloperInputContext) -> bool {
        // We destructure here to ensure that, when new fields are added, we check said fields.
        let UniversalDeveloperInputContextUpdate {
            selected_model: updated_selected_model,
            input_mode: updated_input_mode,
            selected_conversation: updated_selected_conversation,
            auto_approve_agent_actions: updated_auto_approve_agent_actions,
            long_running_command_agent_interaction_state:
                updated_long_running_command_agent_interaction_state,
            long_running_command_agent_interaction: updated_long_running_command_agent_interaction,
            cli_agent_session: updated_cli_agent_session,
        } = self;
        let UniversalDeveloperInputContext {
            selected_model: cached_selected_model,
            input_mode: cached_input_mode,
            selected_conversation: cached_selected_conversation,
            auto_approve_agent_actions: cached_auto_approve_agent_actions,
            long_running_command_agent_interaction_state:
                cached_long_running_command_agent_interaction_state,
            long_running_command_agent_interaction: cached_long_running_command_agent_interaction,
            cli_agent_session: cached_cli_agent_session,
        } = cached;

        // If any of the fields are present and different from the cached context, return true
        // (as the update will change the cached context)
        (updated_selected_model.is_some()
            && updated_selected_model.as_ref() != cached_selected_model.as_ref())
            || (updated_input_mode.is_some()
                && updated_input_mode.as_ref() != cached_input_mode.as_ref())
            || (updated_selected_conversation.is_some()
                && updated_selected_conversation != cached_selected_conversation)
            || (updated_auto_approve_agent_actions.is_some()
                && updated_auto_approve_agent_actions != cached_auto_approve_agent_actions)
            || (updated_long_running_command_agent_interaction_state.is_some()
                && updated_long_running_command_agent_interaction_state
                    != cached_long_running_command_agent_interaction_state)
            || (updated_long_running_command_agent_interaction.is_some()
                && updated_long_running_command_agent_interaction.as_ref()
                    != cached_long_running_command_agent_interaction.as_ref())
            || (updated_cli_agent_session.is_some()
                && updated_cli_agent_session.as_ref() != Some(cached_cli_agent_session))
    }

    /// Merges this update into the current context, returning the new merged state.
    pub fn merge_into(
        self,
        current: UniversalDeveloperInputContext,
    ) -> UniversalDeveloperInputContext {
        let UniversalDeveloperInputContextUpdate {
            selected_model: updated_selected_model,
            input_mode: updated_input_mode,
            selected_conversation: updated_selected_conversation,
            auto_approve_agent_actions: updated_auto_approve_agent_actions,
            long_running_command_agent_interaction_state:
                updated_long_running_command_agent_interaction_state,
            long_running_command_agent_interaction: updated_long_running_command_agent_interaction,
            cli_agent_session: updated_cli_agent_session,
        } = self;
        let UniversalDeveloperInputContext {
            selected_model: current_selected_model,
            input_mode: current_input_mode,
            selected_conversation: current_selected_conversation,
            auto_approve_agent_actions: current_auto_approve_agent_actions,
            long_running_command_agent_interaction_state:
                current_long_running_command_agent_interaction_state,
            long_running_command_agent_interaction: current_long_running_command_agent_interaction,
            cli_agent_session: current_cli_agent_session,
        } = current;

        UniversalDeveloperInputContext {
            selected_model: updated_selected_model.or(current_selected_model),
            input_mode: updated_input_mode.or(current_input_mode),
            selected_conversation: updated_selected_conversation.or(current_selected_conversation),
            auto_approve_agent_actions: updated_auto_approve_agent_actions
                .or(current_auto_approve_agent_actions),
            long_running_command_agent_interaction_state:
                updated_long_running_command_agent_interaction_state
                    .or(current_long_running_command_agent_interaction_state),
            long_running_command_agent_interaction: updated_long_running_command_agent_interaction
                .or(current_long_running_command_agent_interaction),
            cli_agent_session: updated_cli_agent_session.unwrap_or(current_cli_agent_session),
        }
    }
}

impl From<UniversalDeveloperInputContext> for UniversalDeveloperInputContextUpdate {
    fn from(context: UniversalDeveloperInputContext) -> Self {
        Self {
            selected_model: context.selected_model,
            input_mode: context.input_mode,
            selected_conversation: context.selected_conversation,
            auto_approve_agent_actions: context.auto_approve_agent_actions,
            long_running_command_agent_interaction_state: context
                .long_running_command_agent_interaction_state,
            long_running_command_agent_interaction: context.long_running_command_agent_interaction,
            cli_agent_session: Some(context.cli_agent_session),
        }
    }
}
