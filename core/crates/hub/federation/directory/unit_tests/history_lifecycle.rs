use crate::{fixtures::*, history_support::*, support::*};
use yolo_hub_agent_protocol::{Freshness, Reachability};
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol::FIRST_REVISION;
use yolo_hub_terminal_protocol::RuntimeStatus;

#[test]
fn current_pin_discards_old_keys_and_retains_current_key_in_either_restore_order() {
    let entry = Identity::new();
    let old_origin = Identity::new();
    let replacement = rotate(old_origin.identity);
    let old = history(entry.identity, vec![catalogue(&old_origin)]);
    let current = history(entry.identity, vec![catalogue(&replacement)]);
    for histories in [[old.clone(), current.clone()], [current.clone(), old]] {
        let mut book =
            OriginBook::new(entry.identity, vec![replacement.identity], routing_limits()).unwrap();
        for snapshot in histories {
            book.restore_history(snapshot).unwrap();
        }
        assert_eq!(book.history(), current);
        assert_eq!(
            book.pin(replacement.identity.hub()),
            Some(replacement.identity)
        );
        assert!(book.exportable(NOW).unwrap().is_empty());
    }
}

#[test]
fn expired_signed_history_restores_stale_sessions_and_agents_without_routes_or_exports() {
    let fixture = directory::fixture(None);
    let signed = fixture.export.catalogue;
    let expired_at = signed.payload.expires_at_ms + STEP;
    let retained = history(fixture.entry.identity, vec![signed.clone()]);
    let mut book = OriginBook::new(
        fixture.entry.identity,
        vec![fixture.origin.identity],
        routing_limits(),
    )
    .unwrap();
    book.restore_history(retained.clone()).unwrap();
    assert_eq!(book.history(), retained);
    assert_eq!(
        book.latest_for(fixture.origin.identity.hub()).as_deref(),
        Some(&signed)
    );
    let views = book.observations(expired_at).unwrap();
    let view = views.first().unwrap();
    let session = view.terminals.first().unwrap().fields();
    assert_eq!(session.runtime.unwrap().status, RuntimeStatus::Unreachable);
    let agent = view.agents.first().unwrap().fields();
    assert_eq!(agent.freshness, Freshness::Stale);
    assert_eq!(agent.reachability, Reachability::Unreachable);
    assert!(
        book.route_for(fixture.origin.identity.hub(), &observe(), expired_at)
            .unwrap()
            .is_none()
    );
    assert!(book.exportable(expired_at).unwrap().is_empty());
    assert!(
        book.hubs(expired_at)
            .unwrap()
            .iter()
            .all(|hub| !hub.reachable)
    );
}

#[test]
fn an_active_connection_without_any_installed_proof_rejects_history_restoration() {
    let fixture = Fixture::new();
    let retained = history(
        fixture.entry.identity,
        vec![fixture.export.catalogue.clone()],
    );
    let mut book = OriginBook::new(
        fixture.entry.identity,
        vec![fixture.relay.identity, fixture.origin.identity],
        routing_limits(),
    )
    .unwrap();
    let link = uuid::Uuid::new_v4();
    book.connect(fixture.relay.identity, link).unwrap();
    let before = book.history();
    assert!(book.restore_history(retained.clone()).is_err());
    assert_eq!(book.history(), before);
    book.install(
        fixture.relay.identity,
        link,
        FIRST_REVISION,
        vec![fixture.export],
        NOW,
    )
    .unwrap();
    assert!(
        book.route_for(fixture.origin.identity.hub(), &observe(), NOW)
            .unwrap()
            .is_some()
    );
    book.disconnect(fixture.relay.identity.hub(), link).unwrap();
    book.restore_history(retained.clone()).unwrap();
    assert_eq!(book.history(), retained);
}
