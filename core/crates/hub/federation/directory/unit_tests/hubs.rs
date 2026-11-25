use crate::{fixtures::*, inventory::*, support::*};
use uuid::Uuid;
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::{FIRST_REVISION, Purpose, Signed};
use yolo_hub_rpc_protocol::HubSummary;

fn book(fixture: &Fixture) -> OriginBook {
    OriginBook::new(
        fixture.entry.identity,
        vec![fixture.relay.identity, fixture.origin.identity],
        routing_limits(),
    )
    .unwrap()
}

#[test]
fn configured_pins_and_connected_peers_are_not_observed_hubs() {
    let fixture = Fixture::new();
    let mut book = book(&fixture);
    assert!(book.hubs(NOW).unwrap().is_empty());
    let link = Uuid::new_v4();
    book.connect(fixture.relay.identity, link).unwrap();
    assert!(book.hubs(NOW).unwrap().is_empty());
    book.install(fixture.relay.identity, link, FIRST_REVISION, vec![], NOW)
        .unwrap();
    assert!(book.hubs(NOW).unwrap().is_empty());
}

#[test]
fn authenticated_empty_origin_is_visible_and_survives_relay_loss() {
    let fixture = Fixture::new();
    let mut book = book(&fixture);
    let link = Uuid::new_v4();
    book.connect(fixture.relay.identity, link).unwrap();
    book.install(
        fixture.relay.identity,
        link,
        FIRST_REVISION,
        vec![empty_export(&fixture)],
        NOW,
    )
    .unwrap();
    let origin = fixture.origin.identity.hub();
    assert_eq!(
        book.hubs(NOW).unwrap(),
        vec![HubSummary {
            hub_id: origin,
            reachable: true,
            label: None,
        }]
    );
    assert!(
        book.observations(NOW)
            .unwrap()
            .iter()
            .all(|catalogue| catalogue.terminals.is_empty() && catalogue.agents.is_empty())
    );
    book.disconnect(fixture.relay.identity.hub(), link).unwrap();
    assert_eq!(
        book.hubs(NOW).unwrap(),
        vec![HubSummary {
            hub_id: origin,
            reachable: false,
            label: None,
        }]
    );
}

#[test]
fn old_runtime_observation_does_not_mean_the_authorized_hub_route_is_lost() {
    let fixture = Fixture::new();
    let mut book = book(&fixture);
    let mut export = fixture.export.clone();
    let mut catalogue = export.catalogue.payload.clone();
    catalogue.expires_at_ms = NOW;
    export.catalogue = Signed::new(catalogue, Purpose::Catalogue, &fixture.origin).unwrap();
    let link = Uuid::new_v4();
    book.connect(fixture.relay.identity, link).unwrap();
    book.install(
        fixture.relay.identity,
        link,
        FIRST_REVISION,
        vec![export],
        NOW,
    )
    .unwrap();
    assert!(book.hubs(NOW).unwrap().first().unwrap().reachable);
    assert!(!book.hubs(EXPIRES).unwrap().first().unwrap().reachable);
    assert_eq!(
        book.hubs(EXPIRES).unwrap().first().unwrap().hub_id,
        fixture.origin.identity.hub()
    );
}

#[test]
fn two_authenticated_paths_are_deduplicated_and_either_can_keep_the_hub_reachable() {
    let fixture = Fixture::new();
    let mut book = book(&fixture);
    let relay = Uuid::new_v4();
    let direct = Uuid::new_v4();
    book.connect(fixture.relay.identity, relay).unwrap();
    book.install(
        fixture.relay.identity,
        relay,
        FIRST_REVISION,
        vec![fixture.export.clone()],
        NOW,
    )
    .unwrap();
    book.connect(fixture.origin.identity, direct).unwrap();
    book.install(
        fixture.origin.identity,
        direct,
        FIRST_REVISION,
        vec![direct_export(&fixture)],
        NOW,
    )
    .unwrap();
    let reachable = vec![HubSummary {
        hub_id: fixture.origin.identity.hub(),
        reachable: true,
        label: None,
    }];
    assert_eq!(book.hubs(NOW).unwrap(), reachable);
    book.disconnect(fixture.relay.identity.hub(), relay)
        .unwrap();
    assert_eq!(book.hubs(NOW).unwrap(), reachable);
    book.install(
        fixture.origin.identity,
        direct,
        FIRST_REVISION + STEP,
        vec![],
        NOW,
    )
    .unwrap();
    assert_eq!(
        book.hubs(NOW).unwrap(),
        vec![HubSummary {
            hub_id: fixture.origin.identity.hub(),
            reachable: false,
            label: None,
        }]
    );
}

#[test]
fn advertised_unreachability_and_forged_catalogues_cannot_create_live_inventory() {
    let fixture = Fixture::new();
    let mut book = book(&fixture);
    let link = Uuid::new_v4();
    book.connect(fixture.relay.identity, link).unwrap();
    let mut forged = fixture.export.clone();
    forged.catalogue.payload.terminals.clear();
    assert!(
        book.install(
            fixture.relay.identity,
            link,
            FIRST_REVISION,
            vec![forged],
            NOW
        )
        .is_err()
    );
    assert!(book.hubs(NOW).unwrap().is_empty());
    let mut unreachable = fixture.export.clone();
    unreachable.reachable = false;
    book.install(
        fixture.relay.identity,
        link,
        FIRST_REVISION,
        vec![unreachable],
        NOW,
    )
    .unwrap();
    assert_eq!(
        book.hubs(NOW).unwrap(),
        vec![HubSummary {
            hub_id: fixture.origin.identity.hub(),
            reachable: false,
            label: None,
        }]
    );
}
