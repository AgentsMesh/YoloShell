use serde::{Deserialize, Serialize};
use yolo_hub_rpc_protocol::Method;
use yolo_plugin_api::ClientCapability;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisabledFeature {
    pub capability: ClientCapability,
    pub missing_methods: Vec<Method>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityNegotiation {
    pub protocol_matches: bool,
    pub startup_missing: Vec<Method>,
    pub enabled_features: Vec<ClientCapability>,
    pub disabled_features: Vec<DisabledFeature>,
}

impl CapabilityNegotiation {
    pub fn can_start(&self) -> bool {
        self.protocol_matches && self.startup_missing.is_empty()
    }

    pub fn missing_for_startup(&self) -> &[Method] {
        &self.startup_missing
    }

    pub fn supports(&self, capability: ClientCapability) -> bool {
        self.can_start() && self.enabled_features.contains(&capability)
    }
}
