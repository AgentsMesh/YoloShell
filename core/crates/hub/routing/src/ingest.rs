use crate::catalog::{ResourceRoutes, remove_peer_routes};
use crate::{PeerConnection, Resource, ResourceCatalog, Route, RouteAdvertisement, RoutingError};
use std::collections::BTreeSet;
use yolo_hub_identity::HubId;

impl<R: Resource> ResourceCatalog<R> {
    /// Atomically replaces one authenticated connection's complete directory.
    /// The session layer supplies strictly increasing snapshot revisions.
    pub fn replace_from(
        &mut self,
        connection: PeerConnection,
        revision: u64,
        advertisements: Vec<RouteAdvertisement<R>>,
    ) -> Result<(), RoutingError> {
        let peer = self
            .peers
            .get_mut(&connection.peer)
            .ok_or(RoutingError::UnknownPeer)?;
        peer.require_connection(connection)?;
        if revision <= peer.revision {
            return Err(RoutingError::StaleRevision);
        }
        if advertisements.len() > self.limits.max_advertisements_per_peer.get() {
            return Err(RoutingError::AdvertisementLimit);
        }
        let mut incoming = ResourceRoutes::<R>::new();
        for advertisement in advertisements {
            validate_path(
                self.local,
                self.limits.max_path_hubs.get(),
                connection.peer,
                &advertisement,
            )?;
            let ceiling = peer
                .grants
                .get(&advertisement.address.origin())
                .ok_or(RoutingError::UnauthorizedOrigin)?;
            let grant = advertisement.grant.attenuate(ceiling);
            if grant.rights.is_empty() {
                return Err(RoutingError::EmptyRights);
            }
            let path: Vec<_> = std::iter::once(self.local)
                .chain(advertisement.path)
                .collect();
            let route = Route {
                path: path.clone(),
                grant,
            };
            if incoming
                .entry(advertisement.address)
                .or_default()
                .insert(path, route)
                .is_some()
            {
                return Err(RoutingError::DuplicateRoute);
            }
        }
        let additional = incoming
            .keys()
            .filter(|address| !self.agents.contains_key(*address))
            .count();
        if additional
            > self
                .limits
                .max_known_agents
                .get()
                .saturating_sub(self.agents.len())
        {
            return Err(RoutingError::AgentLimit);
        }
        for (address, routes) in &incoming {
            let other_routes = self.agents.get(address).map_or(0, |existing| {
                existing
                    .values()
                    .filter(|route| route.next_hop() != Some(connection.peer))
                    .count()
            });
            if routes.len()
                > self
                    .limits
                    .max_routes_per_agent
                    .get()
                    .saturating_sub(other_routes)
            {
                return Err(RoutingError::RouteLimit);
            }
        }
        remove_peer_routes(&mut self.agents, connection.peer, None);
        for (address, routes) in incoming {
            self.agents.entry(address).or_default().extend(routes);
        }
        peer.revision = revision;
        Ok(())
    }
}

fn validate_path<R: Resource>(
    local: HubId,
    max_path_hubs: usize,
    peer: HubId,
    advertisement: &RouteAdvertisement<R>,
) -> Result<(), RoutingError> {
    let path = &advertisement.path;
    let first = path.first().ok_or(RoutingError::EmptyPath)?;
    if *first != peer {
        return Err(RoutingError::ForgedPeer);
    }
    if path.last() != Some(&advertisement.address.origin()) {
        return Err(RoutingError::ForgedOrigin);
    }
    if path.len() >= max_path_hubs {
        return Err(RoutingError::PathLimit);
    }
    let mut seen = BTreeSet::from([local]);
    if path.iter().any(|hub| !seen.insert(*hub)) {
        return Err(RoutingError::CyclicPath);
    }
    Ok(())
}
