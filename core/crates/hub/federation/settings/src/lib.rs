mod address;
mod constants;
mod file_roots;
mod migration;
pub use address::{PeerAddress, SshProfileRef, validate_socket, validate_ssh_target};
use anyhow::{Result, ensure};
pub use constants::*;
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, num::NonZeroUsize};
use yolo_hub_delegation::{MAX_DELEGATION_DEPTH, PeerIdentity};
use yolo_hub_identity::HubId;
use yolo_hub_routing::{Right, Rights, RoutingLimits};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "migration::ConfigurationWire",
    into = "migration::ConfigurationOut"
)]
pub struct Configuration {
    pub peers: Vec<Peer>,
    pub trusted_origins: Vec<PeerIdentity>,
    pub file_roots: Vec<yolo_hub_file_protocol::FileRootConfiguration>,
    /// How peers that arrive through the local SSH-trusted peer socket are
    /// admitted and what the local origin shares with them.
    pub ssh_peers: SshPeerPolicy,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SshPeerPolicy {
    /// Require an explicit peer pin before accepting an inbound link.
    pub require_registration: bool,
    /// Share this Hub's own origin with SSH-authenticated peers.
    pub share_local: bool,
    pub rights: Rights,
}
impl Default for SshPeerPolicy {
    fn default() -> Self {
        Self {
            require_registration: false,
            share_local: true,
            rights: Rights::all(),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Peer {
    pub identity: PeerIdentity,
    pub dial: Option<PeerAddress>,
    pub consume: bool,
    pub exports: Vec<ExportPolicy>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportPolicy {
    pub origin: HubId,
    pub rights: Rights,
    pub remaining_reexports: u8,
}

impl Configuration {
    pub fn validate(&self, local: PeerIdentity) -> Result<()> {
        file_roots::validate(&self.file_roots)?;
        ensure!(
            !self.ssh_peers.share_local || !self.ssh_peers.rights.is_empty(),
            INVALID
        );
        ensure!(
            self.peers.len() <= MAX_PEERS && self.trusted_origins.len() <= MAX_ORIGINS,
            INVALID
        );
        let mut identities = BTreeMap::new();
        let mut keys = BTreeSet::new();
        for identity in self
            .trusted_origins
            .iter()
            .chain(self.peers.iter().map(|peer| &peer.identity))
        {
            ensure!(
                identity.hub() != local.hub() && identity.key() != local.key(),
                DUPLICATE
            );
            match identities.insert(identity.hub(), *identity) {
                Some(previous) => ensure!(previous == *identity, DUPLICATE),
                None => ensure!(keys.insert(*identity.key()), DUPLICATE),
            }
        }
        ensure!(identities.len() <= MAX_ORIGINS, INVALID);
        let mut peers = BTreeSet::new();
        for peer in &self.peers {
            ensure!(peers.insert(peer.identity.hub()), DUPLICATE);
            if let Some(address) = &peer.dial {
                address.validate()?;
            }
            ensure!(!peer.consume || peer.dial.is_some(), INVALID);
            ensure!(peer.exports.len() <= MAX_ORIGINS, INVALID);
            let mut origins = BTreeSet::new();
            for export in &peer.exports {
                ensure!(origins.insert(export.origin), DUPLICATE);
                ensure!(
                    export.origin == local.hub() || identities.contains_key(&export.origin),
                    UNTRUSTED_ORIGIN
                );
                ensure!(
                    export.remaining_reexports <= MAX_DELEGATION_DEPTH
                        && export
                            .rights
                            .allows(&[Right::Observe].into_iter().collect()),
                    INVALID
                );
            }
        }
        Ok(())
    }
    pub fn pins(&self) -> Vec<PeerIdentity> {
        self.peers
            .iter()
            .map(|peer| peer.identity)
            .chain(self.trusted_origins.iter().copied())
            .map(|pin| (pin.hub(), pin))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect()
    }
    pub fn peer(&self, hub: HubId) -> Option<&Peer> {
        self.peers.iter().find(|peer| peer.identity.hub() == hub)
    }
    pub fn export(&self, peer: HubId, origin: HubId) -> Option<&ExportPolicy> {
        self.peer(peer)?
            .exports
            .iter()
            .find(|policy| policy.origin == origin)
    }
    /// The default policy for SSH-authenticated peers. Explicit per-peer
    /// exports still take precedence; this only covers the local origin.
    pub fn shared_export(&self, origin: HubId, local: HubId) -> Option<ExportPolicy> {
        (self.ssh_peers.share_local && origin == local).then(|| ExportPolicy {
            origin,
            rights: self.ssh_peers.rights.clone(),
            remaining_reexports: 0,
        })
    }
}
pub fn routing_limits() -> RoutingLimits {
    RoutingLimits {
        max_path_hubs: NonZeroUsize::new(usize::from(MAX_DELEGATION_DEPTH) + 2).unwrap(),
        max_peers: NonZeroUsize::new(MAX_PEERS).unwrap(),
        max_origins_per_peer: NonZeroUsize::new(MAX_ORIGINS).unwrap(),
        max_known_agents: NonZeroUsize::new(MAX_ORIGINS).unwrap(),
        max_routes_per_agent: NonZeroUsize::new(MAX_ROUTES_PER_ORIGIN).unwrap(),
        max_advertisements_per_peer: NonZeroUsize::new(MAX_PEER_ADVERTISEMENTS).unwrap(),
    }
}
