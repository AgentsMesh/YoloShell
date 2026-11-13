use super::super::federation_support::exports;
use yolo_hub_federation_settings::ExportPolicy;
use yolo_hub_identity::HubId;
use yolo_hub_routing::Right;

pub const ENTRY_DIRECTORY: &str = "agent-control-entry";
pub const RELAY_DIRECTORY: &str = "agent-control-relay";
pub const ORIGIN_DIRECTORY: &str = "agent-control-origin";
pub const AGENT_NAME: &str = "agent-control-native-hook-agent";
pub const SINGLE_AGENT: usize = 1;

#[derive(Clone, Copy)]
pub enum Delegation {
    Control,
    Attenuated,
}

impl Delegation {
    pub fn to_relay(self, origin: HubId) -> ExportPolicy {
        let mut policy = exports(origin, true);
        if matches!(self, Self::Control) {
            policy.rights = policy
                .rights
                .union(&[Right::AgentControl].into_iter().collect());
        }
        policy
    }
    pub fn to_entry(self, origin: HubId) -> ExportPolicy {
        let mut policy = exports(origin, false);
        match self {
            Self::Control => {
                policy.rights = policy
                    .rights
                    .union(&[Right::AgentControl].into_iter().collect());
            }
            Self::Attenuated => {
                policy.rights = [Right::Observe].into_iter().collect();
            }
        }
        policy
    }
}
