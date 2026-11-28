use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;
use yolo_hub_routing::RoutingLimits;
use yolo_paths::identity::DurableObjectIdentity;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Index {
    pub schema_version: u16,
    pub directory: DurableObjectIdentity,
    pub entry: PeerIdentity,
    pub origins: Vec<HubId>,
}
impl Index {
    pub fn validate(
        &self,
        directory: DurableObjectIdentity,
        entry: PeerIdentity,
        limits: RoutingLimits,
    ) -> Result<()> {
        ensure!(
            self.schema_version == SCHEMA_VERSION && self.entry == entry,
            INVALID
        );
        ensure!(self.directory == directory, CHANGED);
        ensure!(
            self.origins.len() <= limits.max_known_agents.get(),
            CAPACITY
        );
        ensure!(
            self.origins.windows(2).all(|pair| pair[0] < pair[1]),
            DUPLICATE
        );
        ensure!(!self.origins.contains(&entry.hub()), INVALID);
        Ok(())
    }
}
