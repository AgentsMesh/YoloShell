use super::constants::CACHE_SCHEMA;
use anyhow::{Result, ensure};
use serde::Deserialize;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;
use yolo_hub_peer_protocol::{Catalogue, Purpose, Signed};
use yolo_runtime::paths::identity::DurableObjectIdentity;

#[derive(Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Index {
    schema_version: u16,
    directory: DurableObjectIdentity,
    entry: PeerIdentity,
    pub origins: Vec<HubId>,
}
impl Index {
    pub fn validate(&self, owner: PeerIdentity, directory: DurableObjectIdentity) -> Result<()> {
        ensure!(
            self.schema_version == CACHE_SCHEMA && self.entry == owner,
            "invalid cache index identity"
        );
        ensure!(self.directory == directory, "cache index directory changed");
        ensure!(
            self.origins.windows(2).all(|pair| pair[0] < pair[1]),
            "duplicate or unsorted cache origin"
        );
        ensure!(
            !self.origins.contains(&owner.hub()),
            "cache contains its entry origin"
        );
        ensure!(
            self.origins.len()
                <= yolo_hub_federation_settings::routing_limits()
                    .max_known_agents
                    .get(),
            "cache index exceeds origin limit"
        );
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    schema_version: u16,
    directory: DurableObjectIdentity,
    entry: PeerIdentity,
    pub catalogue: Signed<Catalogue>,
}
impl Record {
    pub fn validate(
        &self,
        owner: PeerIdentity,
        directory: DurableObjectIdentity,
        origin: PeerIdentity,
    ) -> Result<()> {
        ensure!(
            self.schema_version == CACHE_SCHEMA && self.entry == owner,
            "invalid cache record identity"
        );
        ensure!(
            self.directory == directory,
            "cache record directory changed"
        );
        self.catalogue.verify(Purpose::Catalogue, origin)?;
        self.catalogue.payload.validate(origin)?;
        Ok(())
    }
}
