//! The session-sharing protocol.
//! All messages defined here are shared across the client and server,
//! and are serialized/deserialized to/from JSON.
//!
//! When modifying them, changes need to be backward-compatible. This usually means
//! defining a default value for every new field added that is used during
//! deserialization when the field is missing.
pub mod common;
pub mod sharer;
pub mod viewer;
