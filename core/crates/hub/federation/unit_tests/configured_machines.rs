use super::list_hubs_with_configured;
use std::collections::{BTreeMap, BTreeSet};
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::{Peer, routing_limits};
use yolo_hub_peer_protocol_test_support::support::NOW;
use yolo_hub_rpc_protocol::Reply;

#[test]
fn configured_peer_is_visible_before_its_first_catalogue() {
    let fixture = yolo_hub_peer_protocol_test_support::Fixture::new();
    let configured = Peer {
        identity: fixture.origin.identity,
        dial: None,
        consume: false,
        exports: Vec::new(),
    };
    let book = OriginBook::new(fixture.entry.identity, vec![], routing_limits()).unwrap();
    let reply = list_hubs_with_configured(
        fixture.entry.identity.hub(),
        &book,
        NOW,
        None,
        &BTreeMap::new(),
        &[configured],
        &BTreeSet::new(),
    )
    .unwrap();
    let Reply::Hubs { hubs, next } = reply else { panic!() };
    assert_eq!(next, None);
    assert!(hubs.iter().any(|hub| {
        hub.hub_id == fixture.origin.identity.hub() && !hub.reachable
    }));
}
