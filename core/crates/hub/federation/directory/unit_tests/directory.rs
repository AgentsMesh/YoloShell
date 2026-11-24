use crate::{fixtures::*, support::*};
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::FIRST_REVISION;
use yolo_hub_terminal_protocol::RuntimeStatus;

#[test]
fn relay_loss_retains_original_resources_without_turning_them_into_exits() {
    let fixture = Fixture::new();
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
    assert!(
        book.route_for(fixture.origin.identity.hub(), &observe(), NOW)
            .unwrap()
            .is_some()
    );
    let before = book.observations(NOW).unwrap();
    book.disconnect(fixture.relay.identity.hub(), link).unwrap();
    let after = book.observations(NOW).unwrap();
    assert_eq!(
        before[0].terminals[0].fields().address,
        after[0].terminals[0].fields().address
    );
    assert_eq!(
        after[0].terminals[0].fields().runtime.unwrap().status,
        RuntimeStatus::Unreachable
    );
    assert!(
        book.route_for(fixture.origin.identity.hub(), &observe(), NOW)
            .unwrap()
            .is_none()
    );
    assert!(book.latest_for(fixture.origin.identity.hub()).is_some());
}

#[test]
fn old_connection_callbacks_and_invalid_signed_snapshots_cannot_replace_current_routes() {
    let fixture = Fixture::new();
    let mut book = OriginBook::new(
        fixture.entry.identity,
        vec![fixture.relay.identity, fixture.origin.identity],
        routing_limits(),
    )
    .unwrap();
    let old = uuid::Uuid::new_v4();
    book.connect(fixture.relay.identity, old).unwrap();
    let new = uuid::Uuid::new_v4();
    book.connect(fixture.relay.identity, new).unwrap();
    assert!(
        book.install(
            fixture.relay.identity,
            old,
            FIRST_REVISION,
            vec![fixture.export.clone()],
            NOW
        )
        .is_err()
    );
    book.install(
        fixture.relay.identity,
        new,
        FIRST_REVISION,
        vec![fixture.export.clone()],
        NOW,
    )
    .unwrap();
    assert!(book.disconnect(fixture.relay.identity.hub(), old).is_err());
    let mut forged = fixture.export.clone();
    forged.catalogue.payload.terminals.clear();
    assert!(
        book.install(
            fixture.relay.identity,
            new,
            FIRST_REVISION + STEP,
            vec![forged],
            NOW
        )
        .is_err()
    );
    assert_eq!(
        book.observations(NOW).unwrap()[0].terminals.len(),
        fixture.export.catalogue.payload.terminals.len()
    );
    assert!(
        book.install(fixture.relay.identity, new, FIRST_REVISION, vec![], NOW)
            .is_err()
    );
}

#[test]
fn transport_trust_does_not_silently_trust_the_origins_behind_a_relay() {
    let fixture = Fixture::new();
    let mut book = OriginBook::new(
        fixture.entry.identity,
        vec![fixture.relay.identity],
        routing_limits(),
    )
    .unwrap();
    assert_eq!(book.allowed_origins(), vec![fixture.relay.identity.hub()]);
    let link = uuid::Uuid::new_v4();
    book.connect(fixture.relay.identity, link).unwrap();
    assert!(
        book.install(
            fixture.relay.identity,
            link,
            FIRST_REVISION,
            vec![fixture.export],
            NOW
        )
        .is_err()
    );
    assert!(book.observations(NOW).unwrap().is_empty());
}

#[test]
fn a_known_hub_id_with_another_key_cannot_clear_an_authenticated_directory() {
    let fixture = Fixture::new();
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
    let wrong_key = yolo_hub_delegation::PeerIdentity::new(
        fixture.relay.identity.hub(),
        *fixture.entry.identity.key(),
    )
    .unwrap();
    assert!(
        book.install(wrong_key, link, FIRST_REVISION + STEP, vec![], NOW)
            .is_err()
    );
    assert!(
        book.route_for(fixture.origin.identity.hub(), &observe(), NOW)
            .unwrap()
            .is_some()
    );
}
