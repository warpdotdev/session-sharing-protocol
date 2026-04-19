use super::{BlockId, ParticipantId};
use byte_unit::Byte;
use serde::{Deserialize, Serialize};

/// The replica ID that a participant's CRDT-compliant input
/// buffer must use. This must be unique across a session.
///
/// The sharer is allowed to choose their own replica ID.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct InputReplicaId(String);

impl From<String> for InputReplicaId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for InputReplicaId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A monotonically increasing sequence number to identify sequential edits for a given buffer.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct InputOperationSeqNo(usize);

impl InputOperationSeqNo {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn advance(&mut self) {
        self.0 += 1;
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl From<usize> for InputOperationSeqNo {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// A [`BufferId`] identifies an instance of the buffer in a session.
/// For example, suppose a session starts with buffer_id=B1.
/// When a command is executed and the buffer is reset, the
/// buffer_id=B2, where B1 != B2.
///
/// Today, a [`BufferId`] masquerades as a [`BlockId`].
#[derive(Clone, Debug, Default, Deserialize, Hash, Serialize, Eq, PartialEq)]
pub struct BufferId(String);

impl std::fmt::Display for BufferId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for BufferId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<BlockId> for BufferId {
    fn from(value: BlockId) -> Self {
        Self(value.to_string())
    }
}

impl From<BufferId> for BlockId {
    fn from(value: BufferId) -> Self {
        value.0.into()
    }
}

/// A CRDT-compliant operation.
/// For now, this is a arbitrary payload that clients should know
/// how to serialize / deserialize. Eventually, this will be a
/// strongly-typed data structure.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CrdtOperation(pub Vec<u8>);

/// A unique identifier for an input operation. Specifically,
/// this uniquely identifies an operation for a specific buffer,
/// for a given participant.
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct InputOperationId {
    /// The participant that made the change.
    pub participant_id: ParticipantId,

    /// The ID of the buffer that this operation was applied to.
    pub buffer_id: BufferId,

    /// A monotonically increasing sequence number to identify sequential edits
    /// for a specific buffer.
    pub op_no: InputOperationSeqNo,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InputUpdate {
    pub id: InputOperationId,

    /// A single input operation consists of a batch
    /// of updates.
    pub ops: Vec<CrdtOperation>,
}

impl InputUpdate {
    pub fn num_bytes(&self) -> Byte {
        self.ops
            .iter()
            .map(|op| op.0.len() as u64)
            .fold(0, u64::saturating_add)
            .into()
    }
}

/// A set of reasons why a request to edit the input might fail.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum InputUpdateFailureReason {
    /// The viewer does not have sufficient permissions.
    InsufficientPermissions,
}
