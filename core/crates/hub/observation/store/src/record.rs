use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_directory::ObservationHistory;
use yolo_hub_peer_protocol::{Catalogue, Signed};
use yolo_hub_routing::RoutingLimits;
use yolo_paths::identity::DurableObjectIdentity;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema_version: u16,
    pub directory: DurableObjectIdentity,
    pub entry: PeerIdentity,
    pub catalogue: Signed<Catalogue>,
}
impl Record {
    pub fn new(
        directory: DurableObjectIdentity,
        entry: PeerIdentity,
        catalogue: &Signed<Catalogue>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            directory,
            entry,
            catalogue: catalogue.clone(),
        }
    }
    pub fn history(
        &self,
        directory: DurableObjectIdentity,
        entry: PeerIdentity,
        limits: RoutingLimits,
    ) -> Result<ObservationHistory> {
        ensure!(
            self.schema_version == SCHEMA_VERSION && self.entry == entry,
            INVALID
        );
        ensure!(self.directory == directory, CHANGED);
        ObservationHistory::new(entry, vec![self.catalogue.clone()], limits)
    }
}
