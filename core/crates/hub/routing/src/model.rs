use crate::{Rights, constants::LOCAL_PATH_HUBS};
use serde::{Deserialize, Serialize};
use yolo_hub_identity::{AgentAddress, HubId, RuntimeEpoch};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub rights: Rights,
    pub remaining_reexports: usize,
}

impl Grant {
    pub fn attenuate(&self, ceiling: &Self) -> Self {
        Self {
            rights: self.rights.intersection(&ceiling.rights),
            remaining_reexports: self.remaining_reexports.min(ceiling.remaining_reexports),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteAdvertisement<R = AgentAddress> {
    pub address: R,
    /// Starts at the authenticated sender and ends at the resource's origin.
    pub path: Vec<HubId>,
    pub grant: Grant,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Route {
    /// Includes this catalog's local Hub first, and the origin last.
    pub path: Vec<HubId>,
    pub grant: Grant,
}

impl Route {
    pub fn next_hop(&self) -> Option<HubId> {
        self.path.get(LOCAL_PATH_HUBS).copied()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogEntry<R = AgentAddress> {
    pub address: R,
    pub reachable: bool,
}

/// Issued only after the caller has authenticated this peer connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeerConnection {
    pub(crate) peer: HubId,
    pub(crate) generation: u64,
    pub(crate) instance: RuntimeEpoch,
}

impl PeerConnection {
    pub fn peer(self) -> HubId {
        self.peer
    }
}
