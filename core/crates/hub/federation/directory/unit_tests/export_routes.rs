use crate::{exports::*, fixtures::*, history_support::resign, inventory::*, support::*};
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::FIRST_REVISION;

#[test]
fn authenticated_exports_choose_the_shortest_complete_path_and_fail_back_after_disconnect() {
    let fixture = Fixture::new();
    let mut book = book(&fixture);
    let relay = fixture.export.clone();
    install(&mut book, &fixture.relay, vec![relay.clone()]);
    assert_exports(&book, vec![relay.clone()], NOW);
    let direct = direct_export(&fixture);
    let link = install(&mut book, &fixture.origin, vec![direct.clone()]);
    assert_exports(&book, vec![direct.clone()], NOW);
    assert_eq!(
        book.route_for(fixture.origin.identity.hub(), &rights(), NOW)
            .unwrap()
            .as_deref(),
        Some(&direct)
    );
    book.disconnect(fixture.origin.identity.hub(), link)
        .unwrap();
    assert_exports(&book, vec![relay], NOW);
}

#[test]
fn equal_length_export_routes_are_deterministic_across_installation_order() {
    let fixture = Fixture::new();
    let other_relay = Identity::new();
    let second = export_via(
        &fixture.origin,
        &fixture.entry,
        &[&other_relay],
        fixture.export.catalogue.clone(),
        rights(),
    );
    let expected = if fixture.relay.identity.hub() < other_relay.identity.hub() {
        fixture.export.clone()
    } else {
        second.clone()
    };
    for reverse in [false, true] {
        let mut book = OriginBook::new(
            fixture.entry.identity,
            vec![
                fixture.origin.identity,
                fixture.relay.identity,
                other_relay.identity,
            ],
            routing_limits(),
        )
        .unwrap();
        let mut paths = [
            (&fixture.relay, fixture.export.clone()),
            (&other_relay, second.clone()),
        ];
        if reverse {
            paths.reverse();
        }
        for (peer, export) in paths {
            install(&mut book, peer, vec![export]);
        }
        assert_exports(&book, vec![expected.clone()], NOW);
    }
}

#[test]
fn grant_expiry_removes_exports_without_discarding_authenticated_observations() {
    let fixture = Fixture::new();
    let mut book = book(&fixture);
    install(&mut book, &fixture.relay, vec![fixture.export.clone()]);
    assert_exports(&book, vec![fixture.export.clone()], EXPIRES - STEP);
    assert_exports(&book, Vec::new(), EXPIRES);
    assert!(book.latest_for(fixture.origin.identity.hub()).is_some());
    assert!(
        book.route_for(fixture.origin.identity.hub(), &observe(), EXPIRES)
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_new_policy_revokes_the_shorter_old_route_and_an_unreachable_export_cannot_replace_it() {
    let fixture = Fixture::new();
    let mut book = book(&fixture);
    install(&mut book, &fixture.origin, vec![direct_export(&fixture)]);
    let mut catalogue = fixture.export.catalogue.payload.clone();
    catalogue.revision += STEP;
    catalogue.policy_epoch += STEP;
    let current = export_via(
        &fixture.origin,
        &fixture.entry,
        &[&fixture.relay],
        resign(catalogue, &fixture.origin),
        rights(),
    );
    let link = install(&mut book, &fixture.relay, vec![current.clone()]);
    assert_exports(&book, vec![current.clone()], NOW);
    let mut unreachable = current;
    unreachable.reachable = false;
    book.install(
        fixture.relay.identity,
        link,
        FIRST_REVISION + STEP,
        vec![unreachable],
        NOW,
    )
    .unwrap();
    assert_exports(&book, Vec::new(), NOW);
    assert!(
        book.route_for(fixture.origin.identity.hub(), &observe(), NOW)
            .unwrap()
            .is_none()
    );
}
