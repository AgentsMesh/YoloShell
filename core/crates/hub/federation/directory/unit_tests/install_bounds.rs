use crate::{
    exports::*,
    fixtures::*,
    history_support::{catalogue, origin_limit},
    support::*,
};
use std::num::NonZeroUsize;
use uuid::Uuid;
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::{FIRST_REVISION, MAX_DIRECTORY_ROUTES};

#[test]
fn duplicate_paths_and_oversized_peer_directories_cannot_replace_a_committed_directory() {
    let fixture = Fixture::new();
    let mut book = book(&fixture);
    let link = install(&mut book, &fixture.relay, vec![fixture.export.clone()]);
    let before = book.history();
    for count in [TWO_PATHS, MAX_DIRECTORY_ROUTES + ONE_PATH] {
        assert!(
            book.install(
                fixture.relay.identity,
                link,
                FIRST_REVISION + STEP,
                vec![fixture.export.clone(); count],
                NOW
            )
            .is_err()
        );
        assert_eq!(book.history(), before);
        assert_exports(&book, vec![fixture.export.clone()], NOW);
    }
    book.install(
        fixture.relay.identity,
        link,
        FIRST_REVISION + STEP,
        vec![fixture.export],
        NOW,
    )
    .unwrap();
}

#[test]
fn aggregate_origin_capacity_rejection_preserves_previous_routes_and_allows_retry() {
    let fixture = Fixture::new();
    let other = Identity::new();
    let mut book = OriginBook::new(
        fixture.entry.identity,
        vec![
            fixture.relay.identity,
            fixture.origin.identity,
            other.identity,
        ],
        origin_limit(ONE_PATH),
    )
    .unwrap();
    let link = install(&mut book, &fixture.relay, vec![fixture.export.clone()]);
    let next_origin = export_via(
        &other,
        &fixture.entry,
        &[&fixture.relay],
        catalogue(&other),
        rights(),
    );
    let before = book.history();
    let next_revision = FIRST_REVISION + STEP;
    assert!(
        book.install(
            fixture.relay.identity,
            link,
            next_revision,
            vec![fixture.export.clone(), next_origin],
            NOW
        )
        .is_err()
    );
    assert_eq!(book.history(), before);
    assert!(book.latest_for(other.identity.hub()).is_none());
    assert_exports(&book, vec![fixture.export.clone()], NOW);
    book.install(
        fixture.relay.identity,
        link,
        next_revision,
        vec![fixture.export],
        NOW,
    )
    .unwrap();
}

#[test]
fn route_capacity_failure_cannot_publish_a_partially_valid_replacement() {
    let fixture = Fixture::new();
    let intermediate = Identity::new();
    let mut limits = routing_limits();
    limits.max_routes_per_agent = NonZeroUsize::new(ONE_PATH).unwrap();
    let mut book = OriginBook::new(
        fixture.entry.identity,
        vec![fixture.relay.identity, fixture.origin.identity],
        limits,
    )
    .unwrap();
    let link = install(&mut book, &fixture.relay, vec![fixture.export.clone()]);
    let longer = export_via(
        &fixture.origin,
        &fixture.entry,
        &[&intermediate, &fixture.relay],
        fixture.export.catalogue.clone(),
        rights(),
    );
    assert!(
        book.install(
            fixture.relay.identity,
            link,
            FIRST_REVISION + STEP,
            vec![fixture.export.clone(), longer],
            NOW
        )
        .is_err()
    );
    assert_exports(&book, vec![fixture.export], NOW);
}

#[test]
fn duplicate_or_local_pins_and_invalid_peer_links_are_rejected_before_state_changes() {
    let fixture = Fixture::new();
    for pins in [
        vec![fixture.entry.identity],
        vec![fixture.origin.identity, fixture.origin.identity],
    ] {
        assert!(OriginBook::new(fixture.entry.identity, pins, routing_limits()).is_err());
    }
    let mut book = book(&fixture);
    let before = book.history();
    assert!(book.connect(fixture.relay.identity, Uuid::nil()).is_err());
    assert!(
        book.connect(Identity::new().identity, Uuid::new_v4())
            .is_err()
    );
    assert!(
        book.disconnect(fixture.relay.identity.hub(), Uuid::new_v4())
            .is_err()
    );
    assert_eq!(book.history(), before);
    install(&mut book, &fixture.relay, vec![fixture.export.clone()]);
    assert_exports(&book, vec![fixture.export], NOW);
}
