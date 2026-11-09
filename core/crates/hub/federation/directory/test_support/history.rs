use crate::support::*;
use std::num::NonZeroUsize;
use yolo_hub_delegation::{FIRST_POLICY_EPOCH, PeerIdentity};
use yolo_hub_federation_directory::{ObservationHistory, OriginBook};
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::{CATALOGUE_SCHEMA_VERSION, Catalogue, FIRST_REVISION, Purpose, Signed};
use yolo_hub_routing::RoutingLimits;

pub const SINGLE_ORIGIN: usize = 1;

pub fn catalogue(origin: &Identity) -> Signed<Catalogue> {
    Signed::new(
        Catalogue {
            schema_version: CATALOGUE_SCHEMA_VERSION,
            revision: FIRST_REVISION,
            policy_epoch: FIRST_POLICY_EPOCH,
            issued_at_ms: BEFORE,
            expires_at_ms: EXPIRES,
            terminals: Vec::new(),
            agents: Vec::new(),
            runs: Vec::new(),
            workspaces: Vec::new(),
        },
        Purpose::Catalogue,
        origin,
    )
    .unwrap()
}

pub fn history(entry: PeerIdentity, catalogues: Vec<Signed<Catalogue>>) -> ObservationHistory {
    ObservationHistory::new(entry, catalogues, routing_limits()).unwrap()
}

pub fn offline(entry: PeerIdentity) -> OriginBook {
    OriginBook::new(entry, Vec::new(), routing_limits()).unwrap()
}

pub fn origin_limit(capacity: usize) -> RoutingLimits {
    RoutingLimits {
        max_known_agents: NonZeroUsize::new(capacity).unwrap(),
        ..routing_limits()
    }
}

pub fn rotate(origin: PeerIdentity) -> Identity {
    let mut replacement = Identity::new();
    replacement.identity = PeerIdentity::new(origin.hub(), *replacement.identity.key()).unwrap();
    replacement
}

pub fn resign(payload: Catalogue, origin: &Identity) -> Signed<Catalogue> {
    Signed::new(payload, Purpose::Catalogue, origin).unwrap()
}

pub fn ordered_origins() -> (Identity, Identity) {
    let first = Identity::new();
    let second = Identity::new();
    if first.identity.hub() < second.identity.hub() {
        (first, second)
    } else {
        (second, first)
    }
}
