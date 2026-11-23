use serde::{Deserialize, Serialize};
use yolo_hub_identity::HubId;

/// Reachability means an authorized Observe route exists, not runtime health.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubSummary {
    pub hub_id: HubId,
    pub reachable: bool,
    /// Operator-facing machine label derived from the entry Hub's peer
    /// configuration. Absent on a Hub's own summary and on unconfigured peers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}
