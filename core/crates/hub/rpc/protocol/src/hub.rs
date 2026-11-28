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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteCapabilities {
    pub lifecycle: bool,
    #[serde(default)]
    pub terminal_read: bool,
    pub terminal_write: bool,
    pub agent_start: bool,
}

impl crate::Reply {
    pub fn catalogue_extensions(watch: bool) -> Self {
        let mut methods = vec![yolo_hub_terminal_protocol::STAGE_ATTACHMENT_METHOD.to_owned(),
            crate::GET_ROUTE_CAPABILITIES_METHOD.to_owned()];
        if watch { methods.push(crate::WATCH_CATALOGUES_METHOD.to_owned()); }
        Self::Extensions { methods }
    }
}
