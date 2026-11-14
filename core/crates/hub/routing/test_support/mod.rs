pub mod constants;

use constants::*;
use std::{collections::BTreeMap, num::NonZeroUsize};
use uuid::Uuid;
use yolo_hub_identity::{AgentAddress, AgentId, HubId};
use yolo_hub_routing::{
    Catalog, Grant, PeerConnection, Right, Rights, RouteAdvertisement, RoutingLimits,
};

pub fn hub(value: u128) -> HubId {
    HubId::from_uuid(Uuid::from_u128(value)).unwrap()
}

pub fn agent(value: u128) -> AgentId {
    AgentId::from_uuid(Uuid::from_u128(value)).unwrap()
}

pub fn address(origin: u128, value: u128) -> AgentAddress {
    AgentAddress {
        hub_id: hub(origin),
        agent_id: agent(value),
    }
}

pub fn limit(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

pub fn limits() -> RoutingLimits {
    RoutingLimits {
        max_path_hubs: limit(MAX_PATH_HUBS),
        max_peers: limit(MAX_PEERS),
        max_origins_per_peer: limit(MAX_ORIGINS),
        max_known_agents: limit(MAX_AGENTS),
        max_routes_per_agent: limit(MAX_ROUTES),
        max_advertisements_per_peer: limit(MAX_ADVERTISEMENTS),
    }
}

pub fn rights() -> Rights {
    [
        Right::Observe,
        Right::TerminalRead,
        Right::TerminalWrite,
        Right::Lifecycle,
        Right::FileRead,
        Right::AgentControl,
    ]
    .into_iter()
    .collect()
}

pub fn observe() -> Rights {
    [Right::Observe].into_iter().collect()
}

pub fn grant() -> Grant {
    Grant {
        rights: rights(),
        remaining_reexports: REEXPORTS,
    }
}

pub fn catalog(local: u128) -> Catalog {
    Catalog::new(hub(local), limits())
}

pub fn connected(catalog: &mut Catalog, peer: u128, origins: &[u128]) -> PeerConnection {
    for origin in origins {
        catalog.authorize(hub(peer), hub(*origin), grant()).unwrap();
    }
    catalog.connect(hub(peer)).unwrap()
}

pub fn advertisement(origin: u128, value: u128, path: &[u128]) -> RouteAdvertisement {
    RouteAdvertisement {
        address: address(origin, value),
        path: path.iter().copied().map(hub).collect(),
        grant: grant(),
    }
}

pub fn policy(origins: &[u128]) -> BTreeMap<HubId, Grant> {
    origins
        .iter()
        .map(|origin| (hub(*origin), grant()))
        .collect()
}
