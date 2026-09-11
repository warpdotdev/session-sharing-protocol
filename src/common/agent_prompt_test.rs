use super::*;
use crate::{sharer, viewer};

fn legacy_request() -> AgentPromptRequest {
    serde_json::from_str(r#"{"id":"req-1","server_conversation_token":null,"prompt":"hello"}"#)
        .unwrap()
}

#[test]
fn legacy_request_has_no_attribution_and_omits_the_field_when_serialized() {
    let request = legacy_request();
    assert!(request.user_query_attribution_b64.is_none());
    let json = serde_json::to_value(request).unwrap();
    assert!(json.get("user_query_attribution_b64").is_none());
}

#[test]
fn envelope_survives_storage_and_both_websocket_directions_opaquely() {
    // Unknown and malformed envelopes belong to the decoder at the destination;
    // the relay must preserve them, including empty values, without dropping input.
    for envelope in ["CgIKABIKCgYKBHVzZXI=", "not-base64!", ""] {
        let mut request = legacy_request();
        request.user_query_attribution_b64 = Some(envelope.to_owned());
        let stored = serde_json::to_string(&request).unwrap();
        let request: AgentPromptRequest = serde_json::from_str(&stored).unwrap();

        let upstream = viewer::UpstreamMessage::SendAgentPrompt(request.clone());
        let upstream = viewer::UpstreamMessage::from_json(&upstream.to_json().unwrap()).unwrap();
        let viewer::UpstreamMessage::SendAgentPrompt(request) = upstream else {
            panic!("expected prompt request");
        };
        assert_eq!(
            request.user_query_attribution_b64.as_deref(),
            Some(envelope)
        );

        let downstream = sharer::DownstreamMessage::AgentPromptRequested {
            id: request.id.clone(),
            participant_id: super::super::ParticipantId::new(),
            request,
        };
        let downstream =
            sharer::DownstreamMessage::from_json(&downstream.to_json().unwrap()).unwrap();
        let sharer::DownstreamMessage::AgentPromptRequested { request, .. } = downstream else {
            panic!("expected prompt delivery");
        };
        assert_eq!(
            request.user_query_attribution_b64.as_deref(),
            Some(envelope)
        );
    }
}

#[test]
fn session_byte_accounting_includes_encoded_envelope_and_inline_content() {
    let mut request = legacy_request();
    request.prompt = "hello 👋".to_owned();
    request.attachments = vec![
        AgentAttachment::PlainText {
            content: "résumé".to_owned(),
        },
        AgentAttachment::FileReference {
            attachment_id: "already-uploaded-file".to_owned(),
            file_name: "file.txt".to_owned(),
        },
    ];
    let baseline_bytes = "hello 👋".len() + "résumé".len();
    assert_eq!(
        viewer::UpstreamMessage::SendAgentPrompt(request.clone())
            .num_bytes()
            .as_u64(),
        baseline_bytes as u64,
    );
    request.user_query_attribution_b64 = Some("CgIKAA==".to_owned());
    assert_eq!(
        viewer::UpstreamMessage::SendAgentPrompt(request)
            .num_bytes()
            .as_u64(),
        (baseline_bytes + "CgIKAA==".len()) as u64,
    );
}
