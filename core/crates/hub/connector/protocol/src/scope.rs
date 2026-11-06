use crate::ConnectorError;
use serde::{Deserialize, Serialize};
use yolo_hub_agent_protocol::ObservedProvider;
use yolo_hub_identity::{HubId, RuntimeTarget};

/// The owner grants exactly this terminal execution and complete provider descriptor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorScope {
    pub terminal: RuntimeTarget,
    pub provider: ObservedProvider,
}

impl ConnectorScope {
    pub fn validate_for(&self, hub: HubId) -> Result<(), ConnectorError> {
        if self.terminal.address.hub_id == hub {
            Ok(())
        } else {
            Err(ConnectorError::InvalidRequest)
        }
    }
}
