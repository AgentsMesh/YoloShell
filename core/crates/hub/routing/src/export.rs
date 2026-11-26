use crate::constants::REEXPORT_STEP;
use crate::{Grant, Resource, ResourceCatalog, RouteAdvertisement, RoutingError};
use std::collections::BTreeMap;
use yolo_hub_identity::HubId;

impl<R: Resource> ResourceCatalog<R> {
    /// Export policy is recipient-specific and must already be authorized.
    /// Transit routes consume one reexport, local publication consumes none.
    pub fn advertisements_for(
        &self,
        recipient: HubId,
        policy: &BTreeMap<HubId, Grant>,
    ) -> Result<Vec<RouteAdvertisement<R>>, RoutingError> {
        if recipient == self.local {
            return Err(RoutingError::LocalPeer);
        }
        let mut output = Vec::new();
        for (address, routes) in &self.agents {
            let Some(ceiling) = policy.get(&address.origin()) else {
                continue;
            };
            for route in routes.values() {
                if route.path.len() >= self.limits.max_path_hubs.get()
                    || route.path.contains(&recipient)
                {
                    continue;
                }
                let mut grant = route.grant.clone();
                if route.next_hop().is_some() {
                    let Some(remaining) = grant.remaining_reexports.checked_sub(REEXPORT_STEP)
                    else {
                        continue;
                    };
                    grant.remaining_reexports = remaining;
                }
                grant = grant.attenuate(ceiling);
                if grant.rights.is_empty() {
                    continue;
                }
                if output.len() >= self.limits.max_advertisements_per_peer.get() {
                    return Err(RoutingError::AdvertisementLimit);
                }
                output.push(RouteAdvertisement {
                    address: *address,
                    path: route.path.clone(),
                    grant,
                });
            }
        }
        Ok(output)
    }
}
