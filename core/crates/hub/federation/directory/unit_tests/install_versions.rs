use crate::{exports::*, fixtures::*, history_support::resign, support::*};
use yolo_hub_peer_protocol::FIRST_REVISION;

#[test]
fn catalogue_revisions_and_policy_epochs_cannot_roll_observations_back() {
    let fixture = Fixture::new();
    let mut current = fixture.export.catalogue.payload.clone();
    current.revision += STEP;
    current.policy_epoch += STEP;
    let retained = resign(current.clone(), &fixture.origin);
    let mut book = book(&fixture);
    let installed = export_via(
        &fixture.origin,
        &fixture.entry,
        &[&fixture.relay],
        retained.clone(),
        rights(),
    );
    let link = install(&mut book, &fixture.relay, vec![installed]);
    let mut link_revision = FIRST_REVISION;
    for (revision, policy, routable) in [
        (current.revision - STEP, current.policy_epoch, true),
        (current.revision - STEP, current.policy_epoch + STEP, false),
        (current.revision + STEP, current.policy_epoch - STEP, false),
    ] {
        let mut stale = current.clone();
        stale.revision = revision;
        stale.policy_epoch = policy;
        let export = export_via(
            &fixture.origin,
            &fixture.entry,
            &[&fixture.relay],
            resign(stale, &fixture.origin),
            rights(),
        );
        link_revision += STEP;
        book.install(
            fixture.relay.identity,
            link,
            link_revision,
            vec![export],
            NOW,
        )
        .unwrap();
        assert_eq!(
            book.latest_for(fixture.origin.identity.hub()).as_deref(),
            Some(&retained)
        );
        assert_eq!(
            book.route_for(fixture.origin.identity.hub(), &observe(), NOW)
                .unwrap()
                .is_some(),
            routable
        );
    }
    for next_policy in [current.policy_epoch, current.policy_epoch + STEP] {
        current.revision += STEP;
        current.policy_epoch = next_policy;
        let updated = resign(current.clone(), &fixture.origin);
        let export = export_via(
            &fixture.origin,
            &fixture.entry,
            &[&fixture.relay],
            updated.clone(),
            rights(),
        );
        link_revision += STEP;
        book.install(
            fixture.relay.identity,
            link,
            link_revision,
            vec![export.clone()],
            NOW,
        )
        .unwrap();
        assert_eq!(
            book.latest_for(fixture.origin.identity.hub()).as_deref(),
            Some(&updated)
        );
        assert_exports(&book, vec![export], NOW);
    }
}

#[test]
fn equal_revision_payload_conflict_preserves_the_previous_catalogue_route_and_link_revision() {
    let fixture = Fixture::new();
    let mut book = book(&fixture);
    let link = install(&mut book, &fixture.relay, vec![fixture.export.clone()]);
    let before = book.history();
    let mut conflicting = fixture.export.clone();
    let mut payload = conflicting.catalogue.payload.clone();
    payload.terminals.clear();
    conflicting.catalogue = resign(payload, &fixture.origin);
    let next = FIRST_REVISION + STEP;
    assert!(
        book.install(fixture.relay.identity, link, next, vec![conflicting], NOW)
            .is_err()
    );
    assert_eq!(book.history(), before);
    assert_exports(&book, vec![fixture.export.clone()], NOW);
    book.install(
        fixture.relay.identity,
        link,
        next,
        vec![fixture.export.clone()],
        NOW,
    )
    .unwrap();
    assert_exports(&book, vec![fixture.export], NOW);
}
