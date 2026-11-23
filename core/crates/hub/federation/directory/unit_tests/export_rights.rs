use crate::{exports::*, fixtures::*, support::*};
use yolo_hub_routing::Right;

#[test]
fn partial_grants_on_two_paths_through_one_peer_cannot_be_combined_into_control_authority() {
    let fixture = Fixture::new();
    let intermediate = Identity::new();
    let read_rights = [Right::Observe, Right::TerminalRead].into_iter().collect();
    let write_rights = [Right::Observe, Right::TerminalWrite].into_iter().collect();
    let read = export_via(
        &fixture.origin,
        &fixture.entry,
        &[&fixture.relay],
        fixture.export.catalogue.clone(),
        read_rights,
    );
    let write = export_via(
        &fixture.origin,
        &fixture.entry,
        &[&intermediate, &fixture.relay],
        fixture.export.catalogue.clone(),
        write_rights,
    );
    let mut book = book(&fixture);
    install(&mut book, &fixture.relay, vec![write.clone(), read.clone()]);
    let origin = fixture.origin.identity.hub();
    assert_eq!(
        book.route_for(origin, &[Right::TerminalRead].into_iter().collect(), NOW)
            .unwrap()
            .as_deref(),
        Some(&read)
    );
    assert_eq!(
        book.route_for(origin, &[Right::TerminalWrite].into_iter().collect(), NOW)
            .unwrap()
            .as_deref(),
        Some(&write)
    );
    assert!(book.route_for(origin, &rights(), NOW).unwrap().is_none());
    assert_exports(&book, vec![read, write], NOW);
}

#[test]
fn a_shorter_observe_only_path_does_not_hide_a_longer_complete_control_path() {
    let fixture = Fixture::new();
    let intermediate = Identity::new();
    let short = export_via(
        &fixture.origin,
        &fixture.entry,
        &[&fixture.relay],
        fixture.export.catalogue.clone(),
        observe(),
    );
    let complete = export_via(
        &fixture.origin,
        &fixture.entry,
        &[&intermediate, &fixture.relay],
        fixture.export.catalogue.clone(),
        rights(),
    );
    let mut book = book(&fixture);
    install(
        &mut book,
        &fixture.relay,
        vec![short.clone(), complete.clone()],
    );
    let origin = fixture.origin.identity.hub();
    assert_eq!(
        book.route_for(origin, &observe(), NOW).unwrap().as_deref(),
        Some(&short)
    );
    assert_eq!(
        book.route_for(origin, &rights(), NOW).unwrap().as_deref(),
        Some(&complete)
    );
    assert_exports(&book, vec![short, complete], NOW);
}
