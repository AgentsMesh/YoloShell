use crate::{
    OriginBook, constants::HISTORY_LOCAL_ORIGIN, exports::*, fixtures::Fixture, history_support::*,
    support::*,
};
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::route;

#[test]
fn history_rejects_an_authentic_catalogue_retained_under_another_hub_key() {
    let entry = Identity::new();
    let origin = Identity::new();
    let other = Identity::new();
    let signed = catalogue(&origin);
    let mut book = offline(entry.identity);
    book.restore_history(history(entry.identity, vec![signed.clone()]))
        .unwrap();
    book.history().validate(routing_limits()).unwrap();

    // Simulate index drift while retaining the original authenticated catalogue.
    let retained = book.latest.remove(&origin.identity.hub()).unwrap();
    assert!(book.latest.insert(other.identity.hub(), retained).is_none());
    let drifted = book.history();
    assert_eq!(drifted.catalogues().next(), Some(&signed));
    assert_eq!(
        drifted.validate(routing_limits()).unwrap_err().to_string(),
        HISTORY_LOCAL_ORIGIN
    );

    let mut restored = offline(entry.identity);
    assert_eq!(
        restored.restore_history(drifted).unwrap_err().to_string(),
        HISTORY_LOCAL_ORIGIN
    );
    assert_eq!(restored.history(), history(entry.identity, Vec::new()));
}

#[test]
fn orphaned_proof_cannot_export_or_route_while_another_owner_remains_available() {
    let fixture = Fixture::new();
    let other = Identity::new();
    let unaffected = export_via(
        &other,
        &fixture.entry,
        &[&fixture.relay],
        catalogue(&other),
        rights(),
    );
    let mut book = OriginBook::new(
        fixture.entry.identity,
        vec![
            fixture.origin.identity,
            fixture.relay.identity,
            other.identity,
        ],
        routing_limits(),
    )
    .unwrap();
    let exports = vec![fixture.export.clone(), unaffected.clone()];
    install(&mut book, &fixture.relay, exports.clone());
    assert_exports(&book, exports.clone(), NOW);

    // A retained proof alone must not substitute for its missing observation.
    let owner = fixture.origin.identity.hub();
    let retained = book.latest.remove(&owner).unwrap();
    assert!(book.proofs.contains_key(&route(&fixture.export.authority)));
    assert!(book.route_for(owner, &observe(), NOW).unwrap().is_none());
    assert_eq!(
        book.route_for(other.identity.hub(), &observe(), NOW)
            .unwrap()
            .as_deref(),
        Some(&unaffected)
    );
    assert_exports(&book, vec![unaffected], NOW);

    assert!(book.latest.insert(owner, retained).is_none());
    assert_exports(&book, exports, NOW);
}
