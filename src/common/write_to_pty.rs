use super::ParticipantId;
use serde::{Deserialize, Serialize};

/// A monotonically increasing sequence number to identify sequential writes to the pty.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct WriteToPtySeqNo(usize);

impl WriteToPtySeqNo {
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

impl From<usize> for WriteToPtySeqNo {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WriteToPtyRequestId {
    pub participant_id: ParticipantId,
    pub op_no: WriteToPtySeqNo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WriteToPtyFailureReason {
    /// The viewer does not have sufficient permissions to write to pty.
    InsufficientPermissions,

    /// The buffer for which the write to pty was requested is old.
    /// Specificaly, there is a new buffer and the command is no longer in-progress.
    StaleBuffer,
}
