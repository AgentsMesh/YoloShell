use super::{
    constants::*,
    fixture::{Fixture, empty_catalogue},
};
use crate::constants::SCHEMA_VERSION;
use std::{fs, num::NonZeroUsize};
use yolo_hub_peer_protocol_test_support::support::Identity;

#[test]
fn incoming_history_above_the_active_limit_is_rejected_before_mutation_without_retiring() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    store.record_history(&fixture.history()).unwrap();
    store.limits.max_known_agents = NonZeroUsize::new(ONE_ORIGIN).unwrap();
    let other = Identity::new();
    let incoming = fixture.with(vec![
        fixture.peer.export.catalogue.clone(),
        empty_catalogue(&other),
    ]);
    let index = fixture.version(&fixture.index_path());
    let record = fixture.version(&fixture.record_path(fixture.peer.origin.identity.hub()));
    assert!(store.record_history(&incoming).is_err());
    assert_eq!(store.history().unwrap(), fixture.history());
    assert_eq!(fixture.version(&fixture.index_path()), index);
    assert_eq!(
        fixture.version(&fixture.record_path(fixture.peer.origin.identity.hub())),
        record
    );
    assert!(!fixture.record_path(other.identity.hub()).exists());
}

#[test]
fn a_matching_retained_index_version_does_not_replace_schema_revalidation() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    store.record_history(&fixture.history()).unwrap();
    let mut invalid = store.index.clone();
    invalid.schema_version = SCHEMA_VERSION + 1;
    let version = store
        .namespace
        .write_json_expected_with_commit_fence(
            &store.path,
            Some(store.index_file),
            &invalid,
            || Ok(()),
        )
        .unwrap();
    // Model a partial initialization retaining an invalid but internally matching pair.
    store.index = invalid;
    store.index_file = version;
    let bytes = fs::read(fixture.index_path()).unwrap();
    assert!(store.history().is_err());
    assert!(store.record_history(&fixture.history()).is_err());
    assert_eq!(fixture.version(&fixture.index_path()), version);
    assert_eq!(fs::read(fixture.index_path()).unwrap(), bytes);
}

#[test]
fn malformed_live_index_or_shard_retires_the_writer_without_repair() {
    for corrupt_index in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        store.record_history(&fixture.history()).unwrap();
        let path = if corrupt_index {
            fixture.index_path()
        } else {
            fixture.record_path(fixture.peer.origin.identity.hub())
        };
        let committed = fs::read(&path).unwrap();
        fs::write(&path, INVALID_BYTES).unwrap();
        assert!(store.history().is_err());
        assert!(store.record_history(&fixture.history()).is_err());
        assert_eq!(fs::read(&path).unwrap(), INVALID_BYTES);
        fs::write(&path, committed).unwrap();
        assert!(store.history().is_err());
        drop(store);
        assert_eq!(
            fixture.open().unwrap().history().unwrap(),
            fixture.history()
        );
    }
}
