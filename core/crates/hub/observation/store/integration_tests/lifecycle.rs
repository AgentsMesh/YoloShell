use crate::{constants::*, fixture::*, integration_helpers::*};
use std::fs;
use yolo_hub_agent_protocol::{Freshness, Reachability};
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol_test_support::support::{Identity, observe};
use yolo_hub_terminal_protocol::RuntimeStatus;
use yolo_paths::constants::MAX_DURABLE_JSON_BYTES;

#[test]
fn multiple_origins_survive_reopen_and_absent_origins_or_empty_snapshots_are_retained() {
    let fixture = Fixture::new();
    let other = Identity::new();
    let first = fixture.peer.export.catalogue.clone();
    let second = empty_catalogue(&other);
    let expected = fixture.with(vec![first.clone(), second]);
    let mut store = fixture.open().unwrap();
    assert_eq!(store.history().unwrap(), fixture.with(Vec::new()));
    store.record_history(&expected).unwrap();
    store.record_history(&fixture.with(vec![first])).unwrap();
    store.record_history(&fixture.with(Vec::new())).unwrap();
    assert_eq!(store.history().unwrap(), expected);
    drop(store);
    assert_eq!(fixture.open().unwrap().history().unwrap(), expected);
}

#[test]
fn unchanged_empty_older_and_lower_policy_snapshots_preserve_all_file_identities() {
    let fixture = Fixture::new();
    let original = &fixture.peer.export.catalogue.payload;
    let revision = original.revision + STEP;
    let policy = original.policy_epoch + STEP;
    let current = fixture.with(vec![fixture.revised(revision, policy)]);
    let mut store = fixture.open().unwrap();
    store.record_history(&current).unwrap();
    let path = fixture.record_path(fixture.peer.origin.identity.hub());
    let index = file_state(&fixture.index_path());
    let record = file_state(&path);
    for snapshot in [
        current.clone(),
        fixture.with(Vec::new()),
        fixture.with(vec![fixture.revised(revision - STEP, policy + STEP)]),
        fixture.with(vec![fixture.revised(revision + STEP, policy - STEP)]),
    ] {
        store.record_history(&snapshot).unwrap();
        assert_eq!(store.history().unwrap(), current);
        assert_eq!(file_state(&fixture.index_path()), index);
        assert_eq!(file_state(&path), record);
    }
}

#[test]
fn newer_empty_catalogue_removes_old_resources_without_forgetting_the_origin() {
    let fixture = committed();
    let mut newer = empty_catalogue(&fixture.peer.origin);
    newer.payload.revision = fixture.peer.export.catalogue.payload.revision + STEP;
    let signed = yolo_hub_peer_protocol::Signed::new(
        newer.payload,
        yolo_hub_peer_protocol::Purpose::Catalogue,
        &fixture.peer.origin,
    )
    .unwrap();
    let expected = fixture.with(vec![signed]);
    let mut store = fixture.open().unwrap();
    let index = file_state(&fixture.index_path());
    store.record_history(&expected).unwrap();
    assert_eq!(file_state(&fixture.index_path()), index);
    drop(store);
    let restored = fixture.open().unwrap().history().unwrap();
    assert_eq!(restored, expected);
    let only = restored.catalogues().next().unwrap();
    assert!(only.payload.terminals.is_empty() && only.payload.agents.is_empty());
}

#[test]
fn same_revision_conflicts_do_not_mutate_or_poison_healthy_storage() {
    let fixture = committed();
    let mut store = fixture.open().unwrap();
    let original = &fixture.peer.export.catalogue.payload;
    let conflict = fixture.with(vec![
        fixture.revised(original.revision, original.policy_epoch + STEP),
    ]);
    let record = fixture.record_path(fixture.peer.origin.identity.hub());
    let before = (file_state(&fixture.index_path()), file_state(&record));
    assert!(store.record_history(&conflict).is_err());
    assert_eq!(store.history().unwrap(), fixture.history());
    assert_eq!(
        (file_state(&fixture.index_path()), file_state(&record)),
        before
    );
    let next = fixture.with(vec![
        fixture.revised(original.revision + STEP, original.policy_epoch),
    ]);
    store.record_history(&next).unwrap();
    assert_eq!(store.history().unwrap(), next);
}

#[test]
fn an_authenticated_replacement_key_can_replace_one_origins_retained_history() {
    let fixture = committed();
    let replacement = rotated(fixture.peer.origin.identity);
    let expected = fixture.with(vec![empty_catalogue(&replacement)]);
    let mut store = fixture.open().unwrap();
    store.record_history(&expected).unwrap();
    drop(store);
    assert_eq!(fixture.open().unwrap().history().unwrap(), expected);
}

#[test]
fn expired_catalogues_restore_only_stale_unreachable_observations() {
    let fixture = committed();
    let expired_at = fixture.peer.export.catalogue.payload.expires_at_ms + STEP;
    let history = fixture.open().unwrap().history().unwrap();
    let mut book = OriginBook::new(
        fixture.peer.entry.identity,
        vec![fixture.peer.origin.identity],
        routing_limits(),
    )
    .unwrap();
    book.restore_history(history).unwrap();
    let observations = book.observations(expired_at).unwrap();
    let view = observations.first().unwrap();
    assert_eq!(
        view.terminals
            .first()
            .unwrap()
            .fields()
            .runtime
            .unwrap()
            .status,
        RuntimeStatus::Unreachable
    );
    let agent = view.agents.first().unwrap().fields();
    assert_eq!(agent.freshness, Freshness::Stale);
    assert_eq!(agent.reachability, Reachability::Unreachable);
    assert!(
        book.route_for(fixture.peer.origin.identity.hub(), &observe(), expired_at)
            .unwrap()
            .is_none()
    );
    assert!(book.exportable(expired_at).unwrap().is_empty());
}

#[test]
fn independent_origin_shards_allow_aggregate_history_larger_than_the_per_file_limit() {
    let fixture = Fixture::new();
    let origins: Vec<_> = (0..BULK_ORIGINS).map(|_| Identity::new()).collect();
    let expected = fixture.with(
        origins
            .iter()
            .map(|origin| large_catalogue(origin, BULK_SESSIONS))
            .collect(),
    );
    let mut store = fixture.open().unwrap();
    store.record_history(&expected).unwrap();
    let sizes: Vec<_> = origins
        .iter()
        .map(|origin| {
            fs::metadata(fixture.record_path(origin.identity.hub()))
                .unwrap()
                .len()
        })
        .collect();
    assert!(sizes.iter().all(|size| *size < MAX_DURABLE_JSON_BYTES));
    assert!(sizes.iter().sum::<u64>() > MAX_DURABLE_JSON_BYTES);
    drop(store);
    assert_eq!(fixture.open().unwrap().history().unwrap(), expected);
}
