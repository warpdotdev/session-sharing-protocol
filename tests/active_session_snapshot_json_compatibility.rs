use session_sharing_protocol::{
    common::{
        ACTIVE_RESTORE_REDUCER_VERSION_V1, ACTIVE_RESTORE_SNAPSHOT_SCHEMA_VERSION_V1,
        ActiveSessionSnapshotCapabilities, ActiveSessionSnapshotCompression,
        ActiveSessionSnapshotPublicationAck, ActiveSessionSnapshotPublicationStatus,
        ActiveSessionSnapshotRestore, ActiveSessionSnapshotResyncReason,
        ActiveSessionSnapshotStorageLayoutKind, FeatureSupport,
        validate_active_session_snapshot_restore,
    },
    sharer, viewer,
};

fn assert_viewer_json_golden_round_trip(json: &str) -> viewer::DownstreamMessage {
    let expected: serde_json::Value = serde_json::from_str(json).expect("valid fixture");
    let message = viewer::DownstreamMessage::from_json(json).expect("protocol decode");
    let actual: serde_json::Value =
        serde_json::from_str(&message.to_json().expect("protocol encode")).expect("encoded JSON");
    assert_eq!(actual, expected);
    message
}

fn assert_sharer_json_golden_round_trip(json: &str) -> sharer::UpstreamMessage {
    let expected: serde_json::Value = serde_json::from_str(json).expect("valid fixture");
    let message = sharer::UpstreamMessage::from_json(json).expect("protocol decode");
    let actual: serde_json::Value =
        serde_json::from_str(&message.to_json().expect("protocol encode")).expect("encoded JSON");
    assert_eq!(actual, expected);
    message
}

fn capabilities() -> ActiveSessionSnapshotCapabilities {
    ActiveSessionSnapshotCapabilities {
        snapshot_schema_versions: vec![ACTIVE_RESTORE_SNAPSHOT_SCHEMA_VERSION_V1],
        reducer_versions: vec![ACTIVE_RESTORE_REDUCER_VERSION_V1],
        storage_layouts: vec![ActiveSessionSnapshotStorageLayoutKind::MonolithicV1],
        compression_codecs: vec![ActiveSessionSnapshotCompression::Zstd],
    }
}

#[test]
fn legacy_rejoin_and_feature_support_json_are_unchanged() {
    let message = assert_viewer_json_golden_round_trip(include_str!(
        "fixtures/rejoined_full_terminal_legacy.json"
    ));
    assert!(!message.requires_active_session_snapshot_support());

    match message {
        viewer::DownstreamMessage::RejoinedSuccessfully {
            active_session_snapshot_restore,
            ..
        } => assert!(active_session_snapshot_restore.is_none()),
        _ => panic!("wrong message"),
    }

    assert_eq!(
        serde_json::to_value(FeatureSupport::default()).expect("feature support"),
        serde_json::json!({
            "supports_agent_view": false,
            "supports_full_role": false,
            "supports_full_role_for_real": false
        })
    );
}

#[test]
fn monolithic_v1_bootstrap_golden_has_an_exact_negotiated_contract() {
    let message =
        assert_viewer_json_golden_round_trip(include_str!("fixtures/rejoined_monolithic_v1.json"));
    assert!(message.requires_active_session_snapshot_support());

    let viewer::DownstreamMessage::RejoinedSuccessfully {
        active_session_snapshot_restore: Some(restore),
        ..
    } = message
    else {
        panic!("wrong message");
    };

    assert!(validate_active_session_snapshot_restore(&capabilities(), &restore).is_ok());
    let restore = *restore;
    let ActiveSessionSnapshotRestore::Bootstrap {
        snapshot,
        resume_from_event_no,
        catch_up_through_event_no,
        retained_event_floor,
        ..
    } = restore
    else {
        panic!("expected bootstrap");
    };
    assert_eq!(snapshot.through_event_no, 41);
    assert_eq!(resume_from_event_no, 42);
    assert_eq!(catch_up_through_event_no, 45);
    assert_eq!(retained_event_floor, 42);
}

#[test]
fn prepared_receipt_is_outside_ordered_event_numbering() {
    let message =
        assert_sharer_json_golden_round_trip(include_str!("fixtures/publish_monolithic_v1.json"));

    let sharer::UpstreamMessage::PublishActiveSessionSnapshot { receipt } = message else {
        panic!("wrong message");
    };
    assert_eq!(receipt.descriptor.through_event_no, 41);
    assert_eq!(receipt.descriptor.snapshot_id, "snapshot-42");
    assert!(!format!("{receipt:?}").contains("opaque-prepared-receipt"));
}

#[test]
fn all_publication_acknowledgements_round_trip() {
    let statuses = [
        ActiveSessionSnapshotPublicationStatus::Prepared,
        ActiveSessionSnapshotPublicationStatus::Committed,
        ActiveSessionSnapshotPublicationStatus::Idempotent,
        ActiveSessionSnapshotPublicationStatus::Stale,
        ActiveSessionSnapshotPublicationStatus::NotContiguousYet,
        ActiveSessionSnapshotPublicationStatus::Conflict,
        ActiveSessionSnapshotPublicationStatus::TooLarge,
        ActiveSessionSnapshotPublicationStatus::UnsupportedVersion,
        ActiveSessionSnapshotPublicationStatus::InvalidObject,
    ];

    for status in statuses {
        let message = sharer::DownstreamMessage::ActiveSessionSnapshotPublicationAck(
            ActiveSessionSnapshotPublicationAck {
                snapshot_id: "snapshot-42".into(),
                status,
                highest_contiguous_event_no: Some(40),
                retained_event_floor: Some(1),
            },
        );
        assert!(message.requires_active_session_snapshot_support());
        let encoded = message.to_json().expect("protocol encode");
        let decoded = sharer::DownstreamMessage::from_json(&encoded).expect("protocol decode");
        let sharer::DownstreamMessage::ActiveSessionSnapshotPublicationAck(ack) = decoded else {
            panic!("wrong message");
        };
        assert_eq!(ack.status, status);
    }
}

#[test]
fn classified_resync_round_trips_and_unknown_reasons_fail_closed() {
    let message = viewer::DownstreamMessage::ActiveSessionSnapshotResyncRequired {
        reason: ActiveSessionSnapshotResyncReason::CursorBelowRetainedFloor {
            retained_event_floor: 42,
        },
    };
    assert!(message.requires_active_session_snapshot_support());
    let encoded = message.to_json().expect("protocol encode");
    assert!(matches!(
        viewer::DownstreamMessage::from_json(&encoded).expect("protocol decode"),
        viewer::DownstreamMessage::ActiveSessionSnapshotResyncRequired {
            reason: ActiveSessionSnapshotResyncReason::CursorBelowRetainedFloor {
                retained_event_floor: 42
            }
        }
    ));
    assert!(
        viewer::DownstreamMessage::from_json(
            r#"{"ActiveSessionSnapshotResyncRequired":{"reason":"NewerUnknownReason"}}"#
        )
        .is_err()
    );
}

#[test]
fn unknown_layout_and_unadvertised_versions_fail_closed() {
    assert!(
        viewer::DownstreamMessage::from_json(
            &include_str!("fixtures/rejoined_monolithic_v1.json")
                .replace("\"MonolithicV1\"", "\"ChunkedV2\"")
        )
        .is_err()
    );

    let message =
        assert_viewer_json_golden_round_trip(include_str!("fixtures/rejoined_monolithic_v1.json"));
    let viewer::DownstreamMessage::RejoinedSuccessfully {
        active_session_snapshot_restore: Some(restore),
        ..
    } = message
    else {
        panic!("wrong message");
    };
    let unsupported = ActiveSessionSnapshotCapabilities {
        snapshot_schema_versions: vec![2],
        ..capabilities()
    };
    assert!(validate_active_session_snapshot_restore(&unsupported, &restore).is_err());
}
