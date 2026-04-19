use super::ParticipantId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum GridType {
    Prompt,
    /// Right side prompt
    Rprompt,
    Output,
    /// Combined prompt/command grid, used for same-line prompt
    PromptAndCommand,
}

/// A point in a grid.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Point {
    pub row: usize,
    pub col: usize,
}

/// A point in a grid within a block.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct BlockPoint {
    pub block_id: BlockId,
    pub grid_type: GridType,
    pub point: Point,
}

/// An ID for a block that is unique only within a single shared session.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockId(String);

impl std::fmt::Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for BlockId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// What a shared session participant has selected.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub enum Selection {
    #[default]
    None,
    Blocks {
        block_ids: Vec<BlockId>,
    },
    /// Start is always before end.
    BlockText {
        start: BlockPoint,
        end: BlockPoint,
        /// If true, the user selected from the end point to the start point (useful for knowing where the cursor should be)
        is_reversed: bool,
    },
    /// Start is always before end
    AltScreenText {
        start: Point,
        end: Point,
        /// If true, the user selected from the end point to the start point (useful for knowing where the cursor should be)
        is_reversed: bool,
    },
}

#[derive(
    Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Deserialize, Serialize,
)]
pub struct SelectionEventNo(usize);

impl From<usize> for SelectionEventNo {
    fn from(value: usize) -> Self {
        Self(value)
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SelectionUpdate {
    pub selection: Selection,
    pub event_no: SelectionEventNo,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PresenceUpdate {
    Selection(Selection),
    // other stuff in the future, like scroll state
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ParticipantPresenceUpdate {
    pub participant_id: ParticipantId,
    pub update: PresenceUpdate,
}

/// One participant's current selection.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ParticipantSelection {
    pub id: ParticipantId,
    pub selection: Selection,
}
