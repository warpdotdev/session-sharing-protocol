use session_sharing_protocol::{
    common::{
        AgentPromptRequest, AgentPromptRequestId, ExecutionIdentity, FeatureSupport,
        OrderedTerminalEventType, SEMANTIC_CONVERSATION_SCHEMA_VERSION_V1, SemanticResyncReason,
        SessionContentMode, validate_negotiated_content,
    },
    sharer, viewer,
};

fn assert_json_golden_round_trip(json: &str) -> viewer::DownstreamMessage {
    let expected: serde_json::Value = serde_json::from_str(json).expect("valid fixture");
    let message = viewer::DownstreamMessage::from_json(json).expect("protocol decode");
    let actual: serde_json::Value =
        serde_json::from_str(&message.to_json().expect("protocol encode")).expect("encoded JSON");
    assert_eq!(actual, expected);
    message
}

#[test]
fn legacy_full_terminal_rejoin_json_is_unchanged() {
    let message =
        assert_json_golden_round_trip(include_str!("fixtures/rejoined_full_terminal_legacy.json"));

    match message {
        viewer::DownstreamMessage::RejoinedSuccessfully {
            negotiated_content, ..
        } => assert!(negotiated_content.is_none()),
        _ => panic!("wrong message"),
    }

    let serialized = serde_json::to_value(FeatureSupport::default()).expect("feature support");
    assert_eq!(
        serialized,
        serde_json::json!({
            "supports_agent_view": false,
            "supports_full_role": false,
            "supports_full_role_for_real": false
        })
    );
}

#[test]
fn agent_prompt_context_is_optional_and_backward_compatible() {
    let legacy: AgentPromptRequest = serde_json::from_value(serde_json::json!({
        "id": "request",
        "server_conversation_token": null,
        "prompt": "continue",
        "attachments": []
    }))
    .expect("legacy prompt");
    assert!(legacy.accepted_message_context.is_none());
    assert!(
        serde_json::to_value(&legacy)
            .expect("legacy prompt encode")
            .get("accepted_message_context")
            .is_none()
    );

    let semantic = AgentPromptRequest {
        id: AgentPromptRequestId::new(),
        server_conversation_token: None,
        prompt: "continue".into(),
        attachments: Vec::new(),
        accepted_message_context: Some(vec![1, 2, 3]),
    };
    let encoded = serde_json::to_value(&semantic).expect("semantic prompt encode");
    assert_eq!(
        encoded.get("accepted_message_context"),
        Some(&serde_json::json!([1, 2, 3]))
    );
    assert_eq!(
        serde_json::from_value::<AgentPromptRequest>(encoded)
            .expect("semantic prompt decode")
            .accepted_message_context,
        semantic.accepted_message_context
    );
}

#[test]
fn storage_unavailable_terminal_reason_is_stable_json() {
    let reason = sharer::SessionTerminatedReason::StorageUnavailable;
    let encoded = serde_json::to_value(&reason).expect("terminal reason encode");
    assert_eq!(encoded, serde_json::json!("StorageUnavailable"));
    assert!(matches!(
        serde_json::from_value::<sharer::SessionTerminatedReason>(encoded)
            .expect("terminal reason decode"),
        sharer::SessionTerminatedReason::StorageUnavailable
    ));
}

#[test]
fn semantic_v1_rejoin_json_echoes_the_exact_contract() {
    let message = assert_json_golden_round_trip(include_str!("fixtures/rejoined_semantic_v1.json"));
    let requested_execution = ExecutionIdentity {
        conversation_id: "conversation".into(),
        execution_id: "execution".into(),
        run_id: Some("run".into()),
        request_id: None,
    };

    match message {
        viewer::DownstreamMessage::RejoinedSuccessfully {
            negotiated_content, ..
        } => assert!(
            validate_negotiated_content(
                SessionContentMode::SemanticConversationOnly,
                Some(1),
                Some(&requested_execution),
                negotiated_content.as_ref(),
            )
            .is_ok()
        ),
        _ => panic!("wrong message"),
    }
}

#[test]
fn semantic_ordered_event_json_scopes_the_cursor() {
    let message =
        assert_json_golden_round_trip(include_str!("fixtures/semantic_ordered_event_v1.json"));

    let viewer::DownstreamMessage::OrderedTerminalEvent(event) = message else {
        panic!("wrong message");
    };
    let OrderedTerminalEventType::SemanticConversationMutation { cursor, mutation } =
        event.event_type
    else {
        panic!("wrong event type");
    };
    assert_eq!(cursor.conversation_id, "conversation");
    assert_eq!(cursor.execution_id, "execution");
    assert_eq!(
        cursor.content_mode,
        SessionContentMode::SemanticConversationOnly
    );
    assert_eq!(
        cursor.schema_version,
        SEMANTIC_CONVERSATION_SCHEMA_VERSION_V1
    );
    assert_eq!(cursor.mutation_sequence, 1);
    assert_eq!(mutation, [1, 2, 3]);
}

#[test]
fn storage_unavailable_resync_is_typed_and_newer_reasons_fail_closed() {
    let message = viewer::DownstreamMessage::SemanticResyncRequired {
        reason: SemanticResyncReason::StorageUnavailable,
    };
    let encoded = message.to_json().expect("protocol encode");
    assert!(matches!(
        viewer::DownstreamMessage::from_json(&encoded).expect("protocol decode"),
        viewer::DownstreamMessage::SemanticResyncRequired {
            reason: SemanticResyncReason::StorageUnavailable
        }
    ));
    assert!(
        viewer::DownstreamMessage::from_json(
            r#"{"SemanticResyncRequired":{"reason":"NewerUnknownReason"}}"#
        )
        .is_err()
    );
}
