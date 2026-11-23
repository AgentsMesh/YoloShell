use crate::{fixtures::*, inventory::*, support::*};
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::FIRST_REVISION;
use yolo_hub_rpc_protocol::HubSummary;
use yolo_hub_terminal_protocol::RuntimeStatus;

fn observed(fixture: &Fixture) -> OriginBook {
    let mut book = OriginBook::new(
        fixture.entry.identity,
        vec![fixture.relay.identity, fixture.origin.identity],
        routing_limits(),
    )
    .unwrap();
    let link = uuid::Uuid::new_v4();
    book.connect(fixture.relay.identity, link).unwrap();
    book.install(
        fixture.relay.identity,
        link,
        FIRST_REVISION,
        vec![fixture.export.clone()],
        NOW,
    )
    .unwrap();
    book
}

#[test]
fn history_restores_observations_without_links_proofs_or_executable_routes() {
    let fixture = Fixture::new();
    let previous = observed(&fixture);
    let mut next = OriginBook::new(
        fixture.entry.identity,
        vec![fixture.relay.identity, fixture.origin.identity],
        routing_limits(),
    )
    .unwrap();
    next.restore_history(previous.history()).unwrap();
    next.restore_history(previous.history()).unwrap();
    assert_eq!(
        next.hubs(NOW).unwrap(),
        vec![HubSummary {
            hub_id: fixture.origin.identity.hub(),
            reachable: false,
            label: None,
        }]
    );
    let views = next.observations(NOW).unwrap();
    assert_eq!(
        views
            .first()
            .unwrap()
            .terminals
            .first()
            .unwrap()
            .fields()
            .address,
        fixture
            .export
            .catalogue
            .payload
            .terminals
            .first()
            .unwrap()
            .fields()
            .address
    );
    assert_eq!(
        views
            .first()
            .unwrap()
            .terminals
            .first()
            .unwrap()
            .fields()
            .runtime
            .unwrap()
            .status,
        RuntimeStatus::Unreachable
    );
    assert!(
        next.route_for(fixture.origin.identity.hub(), &observe(), NOW)
            .unwrap()
            .is_none()
    );
    assert!(next.exportable(NOW).unwrap().is_empty());
    assert!(
        next.install(
            fixture.relay.identity,
            uuid::Uuid::new_v4(),
            FIRST_REVISION,
            vec![fixture.export.clone()],
            NOW
        )
        .is_err()
    );
    let link = uuid::Uuid::new_v4();
    next.connect(fixture.relay.identity, link).unwrap();
    next.install(
        fixture.relay.identity,
        link,
        FIRST_REVISION,
        vec![fixture.export.clone()],
        NOW,
    )
    .unwrap();
    assert!(
        next.route_for(fixture.origin.identity.hub(), &observe(), NOW)
            .unwrap()
            .is_some()
    );
}

#[test]
fn unpinned_history_is_retained_offline_but_a_changed_key_is_discarded() {
    let fixture = Fixture::new();
    let previous = observed(&fixture);
    let mut removed = OriginBook::new(fixture.entry.identity, vec![], routing_limits()).unwrap();
    removed.restore_history(previous.history()).unwrap();
    assert!(removed.allowed_origins().is_empty());
    assert!(removed.latest_for(fixture.origin.identity.hub()).is_some());
    assert!(
        removed
            .route_for(fixture.origin.identity.hub(), &observe(), NOW)
            .unwrap()
            .is_none()
    );
    let other = Identity::new();
    let rotated = yolo_hub_delegation::PeerIdentity::new(
        fixture.origin.identity.hub(),
        *other.identity.key(),
    )
    .unwrap();
    let mut changed =
        OriginBook::new(fixture.entry.identity, vec![rotated], routing_limits()).unwrap();
    changed.restore_history(previous.history()).unwrap();
    assert!(changed.latest_for(fixture.origin.identity.hub()).is_none());
    assert!(changed.hubs(NOW).unwrap().is_empty());
}

#[test]
fn even_empty_origin_history_requires_fresh_authority_after_restoration() {
    let fixture = Fixture::new();
    let mut previous = OriginBook::new(
        fixture.entry.identity,
        vec![fixture.relay.identity, fixture.origin.identity],
        routing_limits(),
    )
    .unwrap();
    let link = uuid::Uuid::new_v4();
    previous.connect(fixture.relay.identity, link).unwrap();
    previous
        .install(
            fixture.relay.identity,
            link,
            FIRST_REVISION,
            vec![empty_export(&fixture)],
            NOW,
        )
        .unwrap();
    let mut next = OriginBook::new(
        fixture.entry.identity,
        vec![fixture.origin.identity],
        routing_limits(),
    )
    .unwrap();
    next.restore_history(previous.history()).unwrap();
    assert_eq!(
        next.hubs(NOW).unwrap(),
        vec![HubSummary {
            hub_id: fixture.origin.identity.hub(),
            reachable: false,
            label: None,
        }]
    );
    assert!(
        next.observations(NOW)
            .unwrap()
            .first()
            .unwrap()
            .terminals
            .is_empty()
    );
}
