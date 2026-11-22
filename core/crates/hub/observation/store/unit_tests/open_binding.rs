use super::{
    constants::*,
    fixture::{Fixture, write_new},
};
use crate::{ObservationStore, record::Record};
use std::{fs, os::unix::fs::symlink};
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol_test_support::support::Identity;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn a_valid_index_cannot_launder_a_shard_bound_to_another_entry_or_directory() {
    for wrong_entry in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        store.record_history(&fixture.history()).unwrap();
        let origin = fixture.peer.origin.identity.hub();
        let path = fixture.record_path(origin);
        let mut record: Record = store.namespace.read_json(&path).unwrap();
        drop(store);
        if wrong_entry {
            record.entry = Identity::new().identity;
        } else {
            record.directory = PrivateDirectory::open_or_create_strict(&fixture.sibling(CLONE))
                .unwrap()
                .durable_identity()
                .unwrap();
        }
        let bytes = serde_json::to_vec(&record).unwrap();
        fs::write(&path, &bytes).unwrap();
        let before = fixture.version(&path);
        assert!(fixture.open().is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fixture.version(&path), before);
    }
}

#[test]
fn stale_directory_descriptor_cannot_initialize_a_replacement_cache_root() {
    let fixture = Fixture::new();
    let original = PrivateDirectory::open(&fixture.path).unwrap();
    fs::rename(&fixture.path, fixture.sibling(DISPLACED)).unwrap();
    PrivateDirectory::open_or_create_strict(&fixture.path).unwrap();
    assert!(
        ObservationStore::open(original, fixture.peer.entry.identity, routing_limits()).is_err()
    );
    assert!(!fixture.index_path().exists());
}

#[test]
fn an_untrusted_lock_symlink_is_not_followed_or_repaired() {
    let fixture = Fixture::new();
    let target = fixture.sibling(REPLACEMENT);
    write_new(&target, INVALID_BYTES);
    let lock = fixture.path.join(LOCK_FILE);
    symlink(&target, &lock).unwrap();
    assert!(fixture.open().is_err());
    assert_eq!(fs::read_link(&lock).unwrap(), target);
    assert_eq!(fs::read(&target).unwrap(), INVALID_BYTES);
    assert!(!fixture.index_path().exists());
}

#[test]
fn a_valid_index_behind_a_symlink_is_not_followed_and_does_not_keep_the_lock() {
    let fixture = Fixture::new();
    fixture
        .open()
        .unwrap()
        .record_history(&fixture.history())
        .unwrap();
    let index = fixture.index_path();
    let displaced = fixture.sibling(DISPLACED);
    let original = fs::read(&index).unwrap();
    fs::rename(&index, &displaced).unwrap();
    symlink(&displaced, &index).unwrap();
    assert!(fixture.open().is_err());
    assert_eq!(fs::read_link(&index).unwrap(), displaced);
    assert_eq!(fs::read(&displaced).unwrap(), original);

    fs::remove_file(&index).unwrap();
    fs::rename(&displaced, &index).unwrap();
    assert_eq!(
        fixture.open().unwrap().history().unwrap(),
        fixture.history()
    );
}

#[test]
fn a_directory_at_the_index_path_cannot_be_adopted_or_replaced() {
    let fixture = Fixture::new();
    drop(fixture.open().unwrap());
    let index = fixture.index_path();
    let saved_index = fixture.sibling(REPLACEMENT);
    fs::rename(&index, &saved_index).unwrap();
    PrivateDirectory::open_or_create_strict(&index).unwrap();
    let retained = index.join(UNKNOWN);
    write_new(&retained, INVALID_BYTES);
    assert!(fixture.open().is_err());
    assert!(index.is_dir());
    assert_eq!(fs::read(&retained).unwrap(), INVALID_BYTES);

    fs::rename(&index, fixture.sibling(DISPLACED)).unwrap();
    // Restore the known index; absence does not authorize reinitializing a
    // nonempty directory, including retained coordination metadata.
    fs::rename(&saved_index, &index).unwrap();
    let reopened = fixture.open().unwrap_or_else(|error| {
        let entries = fs::read_dir(&fixture.path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        panic!("cache reopen failed: {error:#}; retained entries: {entries:?}");
    });
    assert!(reopened.history().unwrap().catalogues().next().is_none());
}
