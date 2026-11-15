use crate::{Grant, PeerConnection, Resource, Route, RoutingError, RoutingLimits};
use std::collections::BTreeMap;
use yolo_hub_identity::{HubId, RuntimeEpoch};

#[derive(Default)]
pub(crate) struct Peer {
    pub grants: BTreeMap<HubId, Grant>,
    pub connection: Option<PeerConnection>,
    pub revision: u64,
}

pub(crate) type ResourceRoutes<R> = BTreeMap<R, BTreeMap<Vec<HubId>, Route>>;

pub struct ResourceCatalog<R: Resource> {
    pub(crate) local: HubId,
    pub(crate) limits: RoutingLimits,
    pub(crate) peers: BTreeMap<HubId, Peer>,
    pub(crate) agents: ResourceRoutes<R>,
    pub(crate) generation: u64,
    pub(crate) instance: RuntimeEpoch,
}

impl<R: Resource> ResourceCatalog<R> {
    pub fn new(local: HubId, limits: RoutingLimits) -> Self {
        Self {
            local,
            limits,
            peers: BTreeMap::new(),
            agents: BTreeMap::new(),
            generation: crate::constants::INITIAL_REVISION,
            instance: RuntimeEpoch::new(),
        }
    }

    pub fn local_hub(&self) -> HubId {
        self.local
    }

    pub fn register_local(
        &mut self,
        local_id: R::LocalId,
        grant: Grant,
    ) -> Result<R, RoutingError> {
        if grant.rights.is_empty() {
            return Err(RoutingError::EmptyRights);
        }
        let address = R::local(self.local, local_id);
        if address.origin() != self.local {
            return Err(RoutingError::ForgedOrigin);
        }
        if !self.agents.contains_key(&address)
            && self.agents.len() >= self.limits.max_known_agents.get()
        {
            return Err(RoutingError::AgentLimit);
        }
        let path = vec![self.local];
        self.agents
            .entry(address)
            .or_default()
            .insert(path.clone(), Route { path, grant });
        Ok(address)
    }

    pub fn unregister_local(&mut self, local_id: R::LocalId) -> bool {
        let address = R::local(self.local, local_id);
        if address.origin() != self.local {
            return false;
        }
        self.agents.get_mut(&address).is_some_and(|routes| {
            let existed = !routes.is_empty();
            routes.clear();
            existed
        })
    }

    /// Forgetting is explicit: a missing route alone is not resource deletion.
    pub fn forget_offline(&mut self, address: R) -> bool {
        if self.agents.get(&address).is_some_and(BTreeMap::is_empty) {
            self.agents.remove(&address);
            true
        } else {
            false
        }
    }
}

pub(crate) fn remove_peer_routes<R: Resource>(
    agents: &mut ResourceRoutes<R>,
    peer: HubId,
    origin: Option<HubId>,
) {
    for (address, routes) in agents {
        if origin.is_none_or(|origin| origin == address.origin()) {
            routes.retain(|_, route| route.next_hop() != Some(peer));
        }
    }
}
