use super::*;
use crate::{sharer, viewer};

fn legacy_request() -> AgentPromptRequest {
    serde_json::from_str(r#"{"id":"req-1","server_conversation_token":null,"prompt":"hello"}"#)
        .unwrap()
}

#[test]
fn legacy_request_has_no_user_query_and_omits_the_field_when_serialized() {
    let request = legacy_request();
    assert!(request.user_query_b64.is_none());
    let json = serde_json::to_value(request).unwrap();
    assert!(json.get("user_query_b64").is_none());
}

#[test]
fn user_query_round_trips_through_json() {
    let mut request = legacy_request();
    request.user_query_b64 = Some("CgVoZWxsbw==".to_owned());
    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["user_query_b64"], "CgVoZWxsbw==");
    assert_eq!(json["prompt"], "hello");
    let decoded: AgentPromptRequest = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.user_query_b64.as_deref(), Some("CgVoZWxsbw=="));
    assert_eq!(decoded.prompt, "hello");
}

#[test]
fn user_query_survives_storage_and_both_websocket_directions_opaquely() {
    // Unknown and malformed payloads belong to the decoder at the destination;
    // the relay must preserve them, including empty values, without dropping input.
    for encoded in ["CgVoZWxsbxoCCAE=", "not-base64!", ""] {
        let mut request = legacy_request();
        request.user_query_b64 = Some(encoded.to_owned());
        let stored = serde_json::to_string(&request).unwrap();
        let request: AgentPromptRequest = serde_json::from_str(&stored).unwrap();

        let upstream = viewer::UpstreamMessage::SendAgentPrompt(request.clone());
        let upstream = viewer::UpstreamMessage::from_json(&upstream.to_json().unwrap()).unwrap();
        let viewer::UpstreamMessage::SendAgentPrompt(request) = upstream else {
            panic!("expected prompt request");
        };
        assert_eq!(request.user_query_b64.as_deref(), Some(encoded));

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
        assert_eq!(request.user_query_b64.as_deref(), Some(encoded));
    }
}

#[test]
fn session_byte_accounting_includes_encoded_user_query_and_inline_content() {
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
    request.user_query_b64 = Some("CgVoZWxsbw==".to_owned());
    assert_eq!(
        viewer::UpstreamMessage::SendAgentPrompt(request)
            .num_bytes()
            .as_u64(),
        (baseline_bytes + "CgVoZWxsbw==".len()) as u64,
    );
}
