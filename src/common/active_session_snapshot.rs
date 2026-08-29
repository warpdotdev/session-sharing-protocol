use serde::{Deserialize, Serialize};

use super::SessionId;

/// Initial schema version for active-session restore snapshots.
pub const ACTIVE_RESTORE_SNAPSHOT_SCHEMA_VERSION_V1: u32 = 1;

/// Initial reducer version for active-session restore snapshots.
pub const ACTIVE_RESTORE_REDUCER_VERSION_V1: u32 = 1;

/// Storage layouts a client can read.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ActiveSessionSnapshotStorageLayoutKind {
    MonolithicV1,
}

/// Compression codecs supported for immutable snapshot objects.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ActiveSessionSnapshotCompression {
    Zstd,
}

/// Snapshot capabilities advertised by a client.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ActiveSessionSnapshotCapabilities {
    pub snapshot_schema_versions: Vec<u32>,
    pub reducer_versions: Vec<u32>,
    pub storage_layouts: Vec<ActiveSessionSnapshotStorageLayoutKind>,
    pub compression_codecs: Vec<ActiveSessionSnapshotCompression>,
}

impl ActiveSessionSnapshotCapabilities {
    pub fn supports(&self, protocol: &NegotiatedActiveSessionSnapshotProtocol) -> bool {
        self.snapshot_schema_versions
            .contains(&protocol.snapshot_schema_version)
            && self.reducer_versions.contains(&protocol.reducer_version)
            && self.storage_layouts.contains(&protocol.storage_layout)
            && self
                .compression_codecs
                .contains(&protocol.compression_codec)
    }
}

/// Snapshot protocol values positively selected by the server.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct NegotiatedActiveSessionSnapshotProtocol {
    pub snapshot_schema_version: u32,
    pub reducer_version: u32,
    pub storage_layout: ActiveSessionSnapshotStorageLayoutKind,
    pub compression_codec: ActiveSessionSnapshotCompression,
}

/// Exact session and active execution represented by a snapshot.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ActiveSessionSnapshotIdentity {
    pub session_id: SessionId,
    pub conversation_id: String,
    /// Opaque server-issued execution epoch. Clients must not derive this value.
    pub execution_id: String,
    /// Stable conversation-run correlation key across execution handoffs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

/// A protocol selection bound to the exact execution being restored.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct NegotiatedActiveSessionSnapshot {
    pub protocol: NegotiatedActiveSessionSnapshotProtocol,
    pub identity: ActiveSessionSnapshotIdentity,
}

/// The downloaded logical snapshot payload.
///
/// Storage metadata is deliberately outside this payload so its hash does not
/// recursively include itself.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ActiveRestoreSnapshot {
    pub snapshot_id: String,
    pub identity: ActiveSessionSnapshotIdentity,
    pub snapshot_schema_version: u32,
    pub reducer_version: u32,
    pub through_event_no: u64,
    pub captured_at_unix_ms: u64,
    pub ordered_message_ids: Vec<String>,
    pub terminal_state: Vec<u8>,
    pub conversation_data: Vec<u8>,
    pub additional_reducer_state: Vec<u8>,
}

/// An immutable object containing a compressed active-session snapshot.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ImmutableActiveSessionSnapshotObject {
    /// Opaque storage-system identifier. This is not a download URL.
    pub object_id: String,
    /// Opaque immutable object generation.
    pub object_generation: String,
    /// Lowercase hexadecimal SHA-256 of the compressed object bytes.
    pub sha256: String,
    pub compressed_size_bytes: u64,
    pub uncompressed_size_bytes: u64,
    pub compression_codec: ActiveSessionSnapshotCompression,
}

/// Layout-specific storage descriptor for an active-session snapshot.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ActiveSessionSnapshotStorage {
    MonolithicV1 {
        object: ImmutableActiveSessionSnapshotObject,
    },
}

impl ActiveSessionSnapshotStorage {
    pub fn kind(&self) -> ActiveSessionSnapshotStorageLayoutKind {
        match self {
            Self::MonolithicV1 { .. } => ActiveSessionSnapshotStorageLayoutKind::MonolithicV1,
        }
    }

    pub fn compression_codec(&self) -> ActiveSessionSnapshotCompression {
        match self {
            Self::MonolithicV1 { object } => object.compression_codec,
        }
    }
}

/// Immutable metadata needed to select and download one snapshot generation.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ActiveSessionSnapshotDescriptor {
    pub snapshot_id: String,
    pub identity: ActiveSessionSnapshotIdentity,
    pub snapshot_schema_version: u32,
    pub reducer_version: u32,
    pub through_event_no: u64,
    pub captured_at_unix_ms: u64,
    pub storage: ActiveSessionSnapshotStorage,
}

impl ActiveSessionSnapshotDescriptor {
    pub fn negotiated_protocol(&self) -> NegotiatedActiveSessionSnapshotProtocol {
        NegotiatedActiveSessionSnapshotProtocol {
            snapshot_schema_version: self.snapshot_schema_version,
            reducer_version: self.reducer_version,
            storage_layout: self.storage.kind(),
            compression_codec: self.storage.compression_codec(),
        }
    }
}

/// A storage-validated snapshot that may be submitted for publication.
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PreparedActiveSessionSnapshotReceipt {
    pub receipt: String,
    pub descriptor: ActiveSessionSnapshotDescriptor,
}

impl std::fmt::Debug for PreparedActiveSessionSnapshotReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedActiveSessionSnapshotReceipt")
            .field("receipt", &"***")
            .field("descriptor", &self.descriptor)
            .finish()
    }
}

/// Result of preparing or publishing a snapshot generation.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ActiveSessionSnapshotPublicationStatus {
    Prepared,
    Committed,
    Idempotent,
    Stale,
    NotContiguousYet,
    Conflict,
    TooLarge,
    UnsupportedVersion,
    InvalidObject,
}

/// Acknowledgement for a snapshot publication attempt.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ActiveSessionSnapshotPublicationAck {
    pub snapshot_id: String,
    pub status: ActiveSessionSnapshotPublicationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highest_contiguous_event_no: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained_event_floor: Option<u64>,
}

/// Cursor supplied by a viewer reconnecting after snapshot restoration.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ActiveSessionSnapshotResumeCursor {
    pub snapshot_id: String,
    pub identity: ActiveSessionSnapshotIdentity,
    pub snapshot_schema_version: u32,
    pub reducer_version: u32,
    pub storage_layout: ActiveSessionSnapshotStorageLayoutKind,
    pub compression_codec: ActiveSessionSnapshotCompression,
    pub last_contiguous_event_no: u64,
}

impl ActiveSessionSnapshotResumeCursor {
    pub fn protocol(&self) -> NegotiatedActiveSessionSnapshotProtocol {
        NegotiatedActiveSessionSnapshotProtocol {
            snapshot_schema_version: self.snapshot_schema_version,
            reducer_version: self.reducer_version,
            storage_layout: self.storage_layout,
            compression_codec: self.compression_codec,
        }
    }
}

/// Restore instructions returned after a viewer joins or reconnects.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ActiveSessionSnapshotRestore {
    Bootstrap {
        negotiated: NegotiatedActiveSessionSnapshot,
        snapshot: Box<ActiveSessionSnapshotDescriptor>,
        resume_from_event_no: u64,
        catch_up_through_event_no: u64,
        retained_event_floor: u64,
    },
    Resume {
        negotiated: NegotiatedActiveSessionSnapshot,
        snapshot_id: String,
        resume_from_event_no: u64,
        catch_up_through_event_no: u64,
        retained_event_floor: u64,
    },
}

/// Why a viewer must discard transient state and request a fresh bootstrap.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ActiveSessionSnapshotResyncReason {
    SessionChanged,
    ConversationChanged,
    ExecutionChanged,
    SnapshotSchemaMismatch {
        expected: u32,
        received: u32,
    },
    ReducerVersionMismatch {
        expected: u32,
        received: u32,
    },
    StorageLayoutMismatch,
    SnapshotIdentityMismatch,
    CursorAhead {
        highest_contiguous_event_no: u64,
    },
    CursorBelowRetainedFloor {
        retained_event_floor: u64,
    },
    SnapshotMissing,
    EventMissing {
        event_no: u64,
    },
    SnapshotHashMismatch,
    SnapshotSizeMismatch,
    SnapshotDecompressionFailed,
    SnapshotDecodeFailed,
    ReplayGap {
        expected_event_no: u64,
        next_available_event_no: u64,
    },
    ConflictingDuplicate {
        event_no: u64,
    },
    StorageUnavailable,
    BufferLimitExceeded,
    BootstrapTimedOut,
}

/// Fail-closed validation error for negotiated snapshot restore instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveSessionSnapshotValidationError {
    UnsupportedProtocol,
    ProtocolMismatch,
    IdentityMismatch,
    SnapshotIdMismatch,
    SnapshotMetadataMismatch,
    InvalidResumeCursor,
}

pub fn validate_active_restore_snapshot(
    descriptor: &ActiveSessionSnapshotDescriptor,
    snapshot: &ActiveRestoreSnapshot,
) -> Result<(), ActiveSessionSnapshotValidationError> {
    if descriptor.snapshot_id != snapshot.snapshot_id {
        return Err(ActiveSessionSnapshotValidationError::SnapshotIdMismatch);
    }
    if descriptor.identity != snapshot.identity {
        return Err(ActiveSessionSnapshotValidationError::IdentityMismatch);
    }
    if descriptor.snapshot_schema_version != snapshot.snapshot_schema_version
        || descriptor.reducer_version != snapshot.reducer_version
        || descriptor.through_event_no != snapshot.through_event_no
        || descriptor.captured_at_unix_ms != snapshot.captured_at_unix_ms
    {
        return Err(ActiveSessionSnapshotValidationError::SnapshotMetadataMismatch);
    }
    Ok(())
}

pub fn validate_active_session_snapshot_restore(
    capabilities: &ActiveSessionSnapshotCapabilities,
    restore: &ActiveSessionSnapshotRestore,
) -> Result<(), ActiveSessionSnapshotValidationError> {
    match restore {
        ActiveSessionSnapshotRestore::Bootstrap {
            negotiated,
            snapshot,
            resume_from_event_no,
            catch_up_through_event_no,
            retained_event_floor,
        } => {
            if !capabilities.supports(&negotiated.protocol) {
                return Err(ActiveSessionSnapshotValidationError::UnsupportedProtocol);
            }
            if snapshot.negotiated_protocol() != negotiated.protocol {
                return Err(ActiveSessionSnapshotValidationError::ProtocolMismatch);
            }
            if snapshot.identity != negotiated.identity {
                return Err(ActiveSessionSnapshotValidationError::IdentityMismatch);
            }
            if snapshot.through_event_no.checked_add(1) != Some(*resume_from_event_no)
                || *resume_from_event_no > catch_up_through_event_no.saturating_add(1)
                || *retained_event_floor > *resume_from_event_no
            {
                return Err(ActiveSessionSnapshotValidationError::InvalidResumeCursor);
            }
        }
        ActiveSessionSnapshotRestore::Resume {
            negotiated,
            resume_from_event_no,
            catch_up_through_event_no,
            retained_event_floor,
            ..
        } => {
            if !capabilities.supports(&negotiated.protocol) {
                return Err(ActiveSessionSnapshotValidationError::UnsupportedProtocol);
            }
            if *resume_from_event_no > catch_up_through_event_no.saturating_add(1)
                || *retained_event_floor > *resume_from_event_no
            {
                return Err(ActiveSessionSnapshotValidationError::InvalidResumeCursor);
            }
        }
    }

    Ok(())
}

pub fn validate_active_session_snapshot_resume(
    cursor: &ActiveSessionSnapshotResumeCursor,
    restore: &ActiveSessionSnapshotRestore,
) -> Result<(), ActiveSessionSnapshotValidationError> {
    let ActiveSessionSnapshotRestore::Resume {
        negotiated,
        snapshot_id,
        resume_from_event_no,
        ..
    } = restore
    else {
        return Err(ActiveSessionSnapshotValidationError::InvalidResumeCursor);
    };

    if snapshot_id != &cursor.snapshot_id {
        return Err(ActiveSessionSnapshotValidationError::SnapshotIdMismatch);
    }
    if negotiated.identity != cursor.identity {
        return Err(ActiveSessionSnapshotValidationError::IdentityMismatch);
    }
    if negotiated.protocol != cursor.protocol() {
        return Err(ActiveSessionSnapshotValidationError::ProtocolMismatch);
    }
    if cursor.last_contiguous_event_no.checked_add(1) != Some(*resume_from_event_no) {
        return Err(ActiveSessionSnapshotValidationError::InvalidResumeCursor);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn identity() -> ActiveSessionSnapshotIdentity {
        ActiveSessionSnapshotIdentity {
            session_id: SessionId::from_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap(),
            conversation_id: "conversation".into(),
            execution_id: "314159".into(),
            run_id: Some("run".into()),
        }
    }

    fn capabilities() -> ActiveSessionSnapshotCapabilities {
        ActiveSessionSnapshotCapabilities {
            snapshot_schema_versions: vec![ACTIVE_RESTORE_SNAPSHOT_SCHEMA_VERSION_V1],
            reducer_versions: vec![ACTIVE_RESTORE_REDUCER_VERSION_V1],
            storage_layouts: vec![ActiveSessionSnapshotStorageLayoutKind::MonolithicV1],
            compression_codecs: vec![ActiveSessionSnapshotCompression::Zstd],
        }
    }

    fn descriptor() -> ActiveSessionSnapshotDescriptor {
        ActiveSessionSnapshotDescriptor {
            snapshot_id: "snapshot".into(),
            identity: identity(),
            snapshot_schema_version: ACTIVE_RESTORE_SNAPSHOT_SCHEMA_VERSION_V1,
            reducer_version: ACTIVE_RESTORE_REDUCER_VERSION_V1,
            through_event_no: 41,
            captured_at_unix_ms: 1_700_000_000_000,
            storage: ActiveSessionSnapshotStorage::MonolithicV1 {
                object: ImmutableActiveSessionSnapshotObject {
                    object_id: "object".into(),
                    object_generation: "7".into(),
                    sha256: "00".repeat(32),
                    compressed_size_bytes: 512,
                    uncompressed_size_bytes: 1024,
                    compression_codec: ActiveSessionSnapshotCompression::Zstd,
                },
            },
        }
    }

    #[test]
    fn monolithic_bootstrap_requires_exact_protocol_and_cursor() {
        let descriptor = descriptor();
        let restore = ActiveSessionSnapshotRestore::Bootstrap {
            negotiated: NegotiatedActiveSessionSnapshot {
                protocol: descriptor.negotiated_protocol(),
                identity: identity(),
            },
            snapshot: Box::new(descriptor),
            resume_from_event_no: 42,
            catch_up_through_event_no: 45,
            retained_event_floor: 42,
        };

        assert!(validate_active_session_snapshot_restore(&capabilities(), &restore).is_ok());
    }

    #[test]
    fn bootstrap_rejects_an_identity_mismatch() {
        let descriptor = descriptor();
        let mut negotiated_identity = identity();
        negotiated_identity.execution_id = "different-execution".into();
        let restore = ActiveSessionSnapshotRestore::Bootstrap {
            negotiated: NegotiatedActiveSessionSnapshot {
                protocol: descriptor.negotiated_protocol(),
                identity: negotiated_identity,
            },
            snapshot: Box::new(descriptor),
            resume_from_event_no: 42,
            catch_up_through_event_no: 45,
            retained_event_floor: 42,
        };

        assert_eq!(
            validate_active_session_snapshot_restore(&capabilities(), &restore),
            Err(ActiveSessionSnapshotValidationError::IdentityMismatch)
        );
    }

    #[test]
    fn downloaded_snapshot_must_match_its_descriptor() {
        let descriptor = descriptor();
        let mut snapshot = ActiveRestoreSnapshot {
            snapshot_id: descriptor.snapshot_id.clone(),
            identity: descriptor.identity.clone(),
            snapshot_schema_version: descriptor.snapshot_schema_version,
            reducer_version: descriptor.reducer_version,
            through_event_no: descriptor.through_event_no,
            captured_at_unix_ms: descriptor.captured_at_unix_ms,
            ordered_message_ids: vec!["message-1".into()],
            terminal_state: vec![1],
            conversation_data: vec![2],
            additional_reducer_state: vec![3],
        };

        assert!(validate_active_restore_snapshot(&descriptor, &snapshot).is_ok());
        snapshot.through_event_no += 1;
        assert_eq!(
            validate_active_restore_snapshot(&descriptor, &snapshot),
            Err(ActiveSessionSnapshotValidationError::SnapshotMetadataMismatch)
        );
    }

    #[test]
    fn resume_requires_the_exact_previous_cursor() {
        let descriptor = descriptor();
        let cursor = ActiveSessionSnapshotResumeCursor {
            snapshot_id: descriptor.snapshot_id.clone(),
            identity: descriptor.identity.clone(),
            snapshot_schema_version: descriptor.snapshot_schema_version,
            reducer_version: descriptor.reducer_version,
            storage_layout: descriptor.storage.kind(),
            compression_codec: descriptor.storage.compression_codec(),
            last_contiguous_event_no: 44,
        };
        let restore = ActiveSessionSnapshotRestore::Resume {
            negotiated: NegotiatedActiveSessionSnapshot {
                protocol: descriptor.negotiated_protocol(),
                identity: descriptor.identity,
            },
            snapshot_id: descriptor.snapshot_id,
            resume_from_event_no: 45,
            catch_up_through_event_no: 47,
            retained_event_floor: 42,
        };

        assert!(validate_active_session_snapshot_resume(&cursor, &restore).is_ok());
    }

    #[test]
    fn unknown_storage_layout_fails_closed() {
        assert!(
            serde_json::from_str::<ActiveSessionSnapshotStorage>(
                r#"{"ChunkedV2":{"manifest":{"object_id":"manifest"}}}"#
            )
            .is_err()
        );
    }
}
