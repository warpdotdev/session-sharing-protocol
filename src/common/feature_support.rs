use super::ActiveSessionSnapshotCapabilities;
use serde::{Deserialize, Serialize};

/// Client feature support declaration.
/// Clients include this in their init payloads to declare which protocol features they support.
/// This allows the server to adapt messages for backward compatibility with older clients.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct FeatureSupport {
    /// Whether the client supports agent view and has it enabled.
    #[serde(default)]
    pub supports_agent_view: bool,
    /// Unused in favor of supports_full_role_for_real. Clients set this to true before they were actually ready.
    #[serde(default)]
    pub supports_full_role: bool,
    /// Whether the client supports the "Full" role ACL.
    #[serde(default)]
    pub supports_full_role_for_real: bool,
    /// Active-session snapshot protocol versions this client can use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_session_snapshot: Option<ActiveSessionSnapshotCapabilities>,
}
