use serde::{Deserialize, Serialize};

use super::SessionId;

/// Determines which ordered content a shared session may contain.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum SessionContentMode {
    /// The legacy terminal protocol, including PTY, input, and agent events.
    #[default]
    FullTerminal,
    /// Only versioned semantic conversation mutations.
    SemanticConversationOnly,
}

impl SessionContentMode {
    pub fn is_full_terminal(&self) -> bool {
        *self == Self::FullTerminal
    }
}

/// Current version of the semantic conversation protobuf schema.
pub const SEMANTIC_CONVERSATION_SCHEMA_VERSION_V1: u32 = 1;

/// Stable identity of the execution carried by a semantic session.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExecutionIdentity {
    pub conversation_id: String,
    pub execution_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// A cursor bound to one negotiated semantic conversation stream.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SemanticCursor {
    pub session_id: SessionId,
    pub conversation_id: String,
    pub execution_id: String,
    pub content_mode: SessionContentMode,
    pub schema_version: u32,
    pub mutation_sequence: u64,
}

/// Values positively accepted and echoed by the server.
///
/// Semantic callers must validate this echo before sending or accepting semantic
/// event variants. A missing echo is only compatible with legacy full-terminal mode.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct NegotiatedSessionContent {
    pub content_mode: SessionContentMode,
    pub semantic_schema_version: u32,
    pub execution_identity: ExecutionIdentity,
}

/// Why a semantic caller must discard transient state and fetch a new bootstrap.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum SemanticResyncReason {
    SessionChanged,
    ExecutionChanged,
    SchemaMismatch {
        expected: u32,
        received: u32,
    },
    CursorSessionMismatch,
    CursorExecutionMismatch,
    CursorAhead {
        highest_contiguous_sequence: u64,
    },
    CursorExpired,
    ReplayGap {
        expected_sequence: u64,
        next_available_sequence: u64,
    },
    ConflictingDuplicate {
        mutation_sequence: u64,
    },
    StorageUnavailable,
    SessionStateUnavailable,
}

/// Fail-closed validation errors for semantic negotiation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticNegotiationError {
    MissingRequestedSchema,
    MissingRequestedExecution,
    MissingServerEcho,
    ContentModeMismatch,
    SchemaMismatch,
    ExecutionMismatch,
}

/// Validates the server echo for a requested content mode.
///
/// Legacy full-terminal callers accept an absent echo; a present echo must retain
/// full-terminal mode. Semantic callers require exact mode, schema, and execution
/// identity matches.
pub fn validate_negotiated_content(
    requested_mode: SessionContentMode,
    requested_schema: Option<u32>,
    requested_execution: Option<&ExecutionIdentity>,
    echoed: Option<&NegotiatedSessionContent>,
) -> Result<(), SemanticNegotiationError> {
    if requested_mode == SessionContentMode::FullTerminal {
        return match echoed {
            Some(echoed) if echoed.content_mode != requested_mode => {
                Err(SemanticNegotiationError::ContentModeMismatch)
            }
            _ => Ok(()),
        };
    }

    let requested_schema =
        requested_schema.ok_or(SemanticNegotiationError::MissingRequestedSchema)?;
    let requested_execution =
        requested_execution.ok_or(SemanticNegotiationError::MissingRequestedExecution)?;
    let echoed = echoed.ok_or(SemanticNegotiationError::MissingServerEcho)?;

    if echoed.content_mode != requested_mode {
        return Err(SemanticNegotiationError::ContentModeMismatch);
    }
    if echoed.semantic_schema_version != requested_schema {
        return Err(SemanticNegotiationError::SchemaMismatch);
    }
    if &echoed.execution_identity != requested_execution {
        return Err(SemanticNegotiationError::ExecutionMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn execution() -> ExecutionIdentity {
        ExecutionIdentity {
            conversation_id: "conversation".into(),
            execution_id: "execution".into(),
            run_id: Some("run".into()),
            request_id: None,
        }
    }

    #[test]
    fn semantic_negotiation_requires_an_exact_echo() {
        assert_eq!(
            validate_negotiated_content(
                SessionContentMode::SemanticConversationOnly,
                Some(1),
                Some(&execution()),
                None,
            ),
            Err(SemanticNegotiationError::MissingServerEcho)
        );

        let echoed = NegotiatedSessionContent {
            content_mode: SessionContentMode::SemanticConversationOnly,
            semantic_schema_version: 1,
            execution_identity: execution(),
        };
        assert!(
            validate_negotiated_content(
                SessionContentMode::SemanticConversationOnly,
                Some(1),
                Some(&execution()),
                Some(&echoed),
            )
            .is_ok()
        );
    }

    #[test]
    fn full_terminal_accepts_only_an_absent_or_matching_echo() {
        assert!(
            validate_negotiated_content(SessionContentMode::FullTerminal, None, None, None).is_ok()
        );

        let matching_echo = NegotiatedSessionContent {
            content_mode: SessionContentMode::FullTerminal,
            semantic_schema_version: 0,
            execution_identity: execution(),
        };
        assert!(
            validate_negotiated_content(
                SessionContentMode::FullTerminal,
                None,
                None,
                Some(&matching_echo),
            )
            .is_ok()
        );

        let mismatched_echo = NegotiatedSessionContent {
            content_mode: SessionContentMode::SemanticConversationOnly,
            semantic_schema_version: 1,
            execution_identity: execution(),
        };
        assert_eq!(
            validate_negotiated_content(
                SessionContentMode::FullTerminal,
                None,
                None,
                Some(&mismatched_echo),
            ),
            Err(SemanticNegotiationError::ContentModeMismatch)
        );
    }

    #[test]
    fn semantic_cursor_round_trips_with_its_full_scope() {
        let cursor = SemanticCursor {
            session_id: SessionId::new(),
            conversation_id: "conversation".into(),
            execution_id: "execution".into(),
            content_mode: SessionContentMode::SemanticConversationOnly,
            schema_version: SEMANTIC_CONVERSATION_SCHEMA_VERSION_V1,
            mutation_sequence: 42,
        };

        let encoded = serde_json::to_string(&cursor).unwrap();
        let decoded = serde_json::from_str(&encoded).unwrap();

        assert_eq!(cursor, decoded);
    }

    #[test]
    fn storage_unavailable_resync_round_trips_and_unknown_reasons_fail_closed() {
        let encoded = serde_json::to_string(&SemanticResyncReason::StorageUnavailable).unwrap();
        assert_eq!(
            serde_json::from_str::<SemanticResyncReason>(&encoded).unwrap(),
            SemanticResyncReason::StorageUnavailable
        );
        assert!(serde_json::from_str::<SemanticResyncReason>("\"NewerUnknownReason\"").is_err());
    }
}
