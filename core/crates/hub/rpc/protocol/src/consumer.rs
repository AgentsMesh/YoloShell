use serde::{Deserialize, Serialize};
use yolo_hub_identity::HubId;

/// One remote machine currently importing from this Hub.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConsumerSummary {
    pub hub: HubId,
    /// Unix milliseconds when the first live link from this machine arrived.
    pub connected_since_ms: u64,
    /// Number of live links from this machine.
    pub links: u16,
}
