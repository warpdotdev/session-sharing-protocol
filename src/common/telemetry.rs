use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Context about the end-client that we want to attach to telemetry events.
///
/// This is an opaque blob that the client chooses because we want to respect
/// whatever context it wants to declare rather than a custom schema.
/// Since we use Rudderstack, it should still respect the named fields
/// here: https://www.rudderstack.com/docs/event-spec/standard-events/common-fields/#contextual-fields
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryContext(pub Value);
