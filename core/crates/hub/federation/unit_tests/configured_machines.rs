use super::list_hubs_with_configured;
use std::collections::BTreeMap;
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
    )
    .unwrap();
    let Reply::Hubs { hubs, next } = reply else { panic!() };
    assert_eq!(next, None);
    assert!(hubs.iter().any(|hub| {
        hub.hub_id == fixture.origin.identity.hub() && !hub.reachable
    }));
}

#[test]
fn route_capabilities_require_an_authenticated_rights_route() {
    let fixture = yolo_hub_peer_protocol_test_support::Fixture::new();
    let book = OriginBook::new(fixture.entry.identity, vec![], routing_limits()).unwrap();
    let local = book.route_capabilities(fixture.entry.identity.hub(), fixture.entry.identity.hub(), NOW).unwrap();
    assert!(local.lifecycle && local.terminal_write && local.agent_start);
    let remote = book.route_capabilities(fixture.entry.identity.hub(), fixture.origin.identity.hub(), NOW).unwrap();
    assert_eq!(remote, Default::default());
}
