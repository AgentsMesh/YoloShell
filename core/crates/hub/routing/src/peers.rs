use crate::catalog::{Peer, remove_peer_routes};
use crate::constants::{INITIAL_REVISION, NEXT_GENERATION};
use crate::{Grant, PeerConnection, Resource, ResourceCatalog, RoutingError};
use yolo_hub_identity::HubId;

impl<R: Resource> ResourceCatalog<R> {
    /// The caller must verify the origin's delegation to this peer first.
    /// Transport authentication alone must not install transit-origin grants.
    pub fn authorize(
        &mut self,
        peer: HubId,
        origin: HubId,
        grant: Grant,
    ) -> Result<(), RoutingError> {
        if peer == self.local {
            return Err(RoutingError::LocalPeer);
        }
        if origin == self.local {
            return Err(RoutingError::LocalOrigin);
        }
        if grant.rights.is_empty() {
            return Err(RoutingError::EmptyRights);
        }
        if !self.peers.contains_key(&peer) && self.peers.len() >= self.limits.max_peers.get() {
            return Err(RoutingError::PeerLimit);
        }
        if let Some(existing) = self.peers.get(&peer) {
            if !existing.grants.contains_key(&origin)
                && existing.grants.len() >= self.limits.max_origins_per_peer.get()
            {
                return Err(RoutingError::OriginLimit);
            }
        }
        remove_peer_routes(&mut self.agents, peer, Some(origin));
        self.peers
            .entry(peer)
            .or_default()
            .grants
            .insert(origin, grant);
        Ok(())
    }

    pub fn revoke(&mut self, peer: HubId, origin: HubId) -> bool {
        let Some(existing) = self.peers.get_mut(&peer) else {
            return false;
        };
        let removed = existing.grants.remove(&origin).is_some();
        if existing.grants.is_empty() {
            self.peers.remove(&peer);
        }
        remove_peer_routes(&mut self.agents, peer, Some(origin));
        removed
    }

    pub fn connect(&mut self, peer: HubId) -> Result<PeerConnection, RoutingError> {
        let state = self.peers.get_mut(&peer).ok_or(RoutingError::UnknownPeer)?;
        let generation = self
            .generation
            .checked_add(NEXT_GENERATION)
            .ok_or(RoutingError::GenerationExhausted)?;
        let connection = PeerConnection {
            peer,
            generation,
            instance: self.instance,
        };
        remove_peer_routes(&mut self.agents, peer, None);
        state.connection = Some(connection);
        state.revision = INITIAL_REVISION;
        self.generation = generation;
        Ok(connection)
    }

    pub fn disconnect(&mut self, connection: PeerConnection) -> Result<(), RoutingError> {
        let state = self
            .peers
            .get_mut(&connection.peer)
            .ok_or(RoutingError::UnknownPeer)?;
        state.require_connection(connection)?;
        remove_peer_routes(&mut self.agents, connection.peer, None);
        state.connection = None;
        Ok(())
    }
}

impl Peer {
    pub(crate) fn require_connection(
        &self,
        connection: PeerConnection,
    ) -> Result<(), RoutingError> {
        if self.connection != Some(connection) {
            return Err(RoutingError::StaleConnection);
        }
        Ok(())
    }
}
