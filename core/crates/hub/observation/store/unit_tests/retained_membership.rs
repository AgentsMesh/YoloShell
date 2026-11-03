use super::{
    constants::*,
    fixture::{Fixture, empty_catalogue, write_new},
};
use crate::record::Record;
use std::fs;
use yolo_hub_peer_protocol_test_support::support::Identity;

#[test]
fn partially_initialized_retained_membership_cannot_adopt_an_existing_disk_origin() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    store.record_history(&fixture.history()).unwrap();
    let origin = fixture.peer.origin.identity.hub();
    let before = fixture.version(&fixture.record_path(origin));
    // Fault-inject a missing retained member, preserving the committed index and disk file.
    // A future partial-open/regeneration path must fail rather than acquire its authority here.
    let removed = store.records.remove(&origin).unwrap();
    assert!(store.history().is_err());
    assert_eq!(fixture.version(&fixture.record_path(origin)), before);
    store.records.insert(origin, removed);
    assert!(store.history().is_err());
    assert!(store.record_history(&fixture.history()).is_err());
}

#[test]
fn a_retained_version_for_the_wrong_origin_never_satisfies_index_membership() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    store.record_history(&fixture.history()).unwrap();
    let origin = fixture.peer.origin.identity.hub();
    let path = fixture.record_path(origin);
    let foreign = Identity::new();
    let record = Record::new(
        store.index.directory,
        store.index.entry,
        &empty_catalogue(&foreign),
    );
    let version = store
        .namespace
        .write_json_expected_with_commit_fence(
            &path,
            Some(store.records[&origin].1),
            &record,
            || Ok(()),
        )
        .unwrap();
    // Even a matching typed record/version pair cannot bind another origin's signed data.
    store.records.insert(origin, (record, version));
    let bytes = fs::read(&path).unwrap();
    assert!(store.history().is_err());
    assert!(store.record_history(&fixture.history()).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fixture.version(&path), version);
}

#[test]
fn retained_value_drift_cannot_hide_behind_an_unchanged_file_version() {
    for index_drift in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        store.record_history(&fixture.history()).unwrap();
        let path = fixture.record_path(fixture.peer.origin.identity.hub());
        let before = (
            fixture.version(&path),
            fixture.version(&fixture.index_path()),
        );
        if index_drift {
            store.index.origins.clear();
        } else {
            store
                .records
                .get_mut(&fixture.peer.origin.identity.hub())
                .unwrap()
                .0
                .catalogue
                .payload
                .terminals
                .clear();
        }
        assert!(store.history().is_err());
        assert!(store.record_history(&fixture.history()).is_err());
        assert_eq!(
            (
                fixture.version(&path),
                fixture.version(&fixture.index_path())
            ),
            before
        );
    }
}

#[test]
fn origin_files_swapped_on_disk_are_rejected_on_reopen_without_repair() {
    let fixture = Fixture::new();
    let other = Identity::new();
    let mut store = fixture.open().unwrap();
    let history = fixture.with(vec![
        fixture.peer.export.catalogue.clone(),
        empty_catalogue(&other),
    ]);
    store.record_history(&history).unwrap();
    drop(store);
    let first = fixture.record_path(fixture.peer.origin.identity.hub());
    let second = fixture.record_path(other.identity.hub());
    let staged = fixture.sibling(DISPLACED);
    fs::rename(&first, &staged).unwrap();
    fs::rename(&second, &first).unwrap();
    fs::rename(&staged, &second).unwrap();
    let before = (fs::read(&first).unwrap(), fs::read(&second).unwrap());
    assert!(fixture.open().is_err());
    assert_eq!(
        (fs::read(&first).unwrap(), fs::read(&second).unwrap()),
        before
    );
}

#[test]
fn lock_substitution_retires_cache_access_without_rewriting_observations() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    store.record_history(&fixture.history()).unwrap();
    let index = fixture.version(&fixture.index_path());
    let path = fixture.path.join(LOCK_FILE);
    fs::rename(&path, fixture.sibling(DISPLACED)).unwrap();
    write_new(&path, INVALID_BYTES);
    assert!(store.history().is_err());
    assert!(store.record_history(&fixture.history()).is_err());
    assert_eq!(fixture.version(&fixture.index_path()), index);
    assert_eq!(fs::read(path).unwrap(), INVALID_BYTES);
}

#[test]
fn retained_membership_does_not_replace_signature_revalidation() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    store.record_history(&fixture.history()).unwrap();
    let origin = fixture.peer.origin.identity.hub();
    let path = fixture.record_path(origin);
    let (mut invalid, previous) = store.records[&origin].clone();
    invalid.catalogue.signature.clear();
    let file = store
        .namespace
        .write_json_expected_with_commit_fence(&path, Some(previous), &invalid, || Ok(()))
        .unwrap();
    store.records.insert(origin, (invalid, file));
    let bytes = fs::read(&path).unwrap();
    assert!(store.history().is_err());
    assert!(store.record_history(&fixture.history()).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fixture.version(&path), file);
}
