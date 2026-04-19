use byte_unit::Byte;
use serde::{Deserialize, Serialize};

/// Scrollback is the set of session contents that weren't shared live
/// but are still part of the shared session.
#[derive(Clone, Deserialize, Serialize)]
pub struct Scrollback {
    /// The blocks that make up the scrollback. Clients are expected
    /// to be able to serialize and deserialize accordingly.
    pub blocks: Vec<ScrollbackBlock>,

    /// True iff the session is in alt-screen mode
    /// at time of share.
    pub is_alt_screen_active: bool,
}

impl Scrollback {
    pub fn num_bytes(&self) -> Byte {
        self.blocks
            .iter()
            .map(|b| b.num_bytes().as_u64())
            .fold(0, u64::saturating_add)
            .into()
    }

    /// Returns true if the scrollback size exceeds |size_bytes|.
    pub fn exceeds_size_bytes(&self, size_bytes: Byte) -> bool {
        self.num_bytes() > size_bytes
    }
}

/// Override the Debug impl to avoid accidentally leaking sensitive
/// data in logs.
impl std::fmt::Debug for Scrollback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Scrollback {{ num_blocks: {}, is_alt_screen_active: {} }}",
            self.blocks.len(),
            self.is_alt_screen_active
        )
    }
}

/// An individual scrollback block.
#[derive(Clone, Deserialize, Serialize)]
pub struct ScrollbackBlock {
    /// The raw contents of the block. Clients are expected to be able to
    /// serialize and deserialize from [`SerializedBlock`] in the Warp client.
    pub raw: Vec<u8>,
}

impl ScrollbackBlock {
    pub fn num_bytes(&self) -> Byte {
        self.raw.len().into()
    }
}

/// Override the Debug impl to avoid accidentally leaking sensitive
/// data in logs.
impl std::fmt::Debug for ScrollbackBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ScrollbackBlock {{ num_bytes: {} }}", self.num_bytes())
    }
}
