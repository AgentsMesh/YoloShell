use crate::{CatalogEntry, Resource, ResourceCatalog, Rights, Route};
use std::collections::BTreeSet;
use yolo_hub_identity::HubId;

impl<R: Resource> ResourceCatalog<R> {
    pub fn entries(&self) -> Vec<CatalogEntry<R>> {
        self.agents
            .iter()
            .map(|(address, routes)| CatalogEntry {
                address: *address,
                reachable: !routes.is_empty(),
            })
            .collect()
    }

    /// Origins with at least one reachable resource, not inferred remote peers.
    pub fn reachable_origins(&self) -> BTreeSet<HubId> {
        self.agents
            .iter()
            .filter(|(_, routes)| !routes.is_empty())
            .map(|(address, _)| address.origin())
            .collect()
    }

    pub fn route_for(&self, address: R, required: &Rights) -> Option<&Route> {
        self.agents
            .get(&address)?
            .values()
            .filter(|route| route.grant.rights.allows(required))
            .min_by(|left, right| {
                left.path
                    .len()
                    .cmp(&right.path.len())
                    .then(left.path.cmp(&right.path))
            })
    }
}
