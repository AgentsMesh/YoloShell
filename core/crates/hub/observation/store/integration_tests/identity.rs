use crate::{constants::*, fixture::*, integration_constants::*, integration_helpers::*};
use std::fs;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_identity::HubId;
use yolo_hub_observation_store::ObservationStore;
use yolo_hub_peer_protocol_test_support::support::Identity;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn concurrent_openers_cannot_share_generation_ownership_and_drop_allows_a_successor() {
    let fixture = committed();
    let store = fixture.open().unwrap();
    std::thread::scope(|scope| {
        let contenders: Vec<_> = (0..CONCURRENT_OPENERS)
            .map(|_| scope.spawn(|| fixture.open().is_err()))
            .collect();
        assert!(contenders.into_iter().all(|thread| thread.join().unwrap()));
    });
    assert_eq!(store.history().unwrap(), fixture.history());
    drop(store);
    assert_eq!(
        fixture.open().unwrap().history().unwrap(),
        fixture.history()
    );
}

#[test]
fn copied_cache_root_cannot_rebind_persisted_directory_identity() {
    let fixture = committed();
    let destination = fixture.sibling(CLONE);
    let before = file_state(&fixture.index_path());
    clone_records(&fixture, &destination);
    assert!(
        ObservationStore::open(
            PrivateDirectory::open(&destination).unwrap(),
            fixture.peer.entry.identity,
            routing_limits()
        )
        .is_err()
    );
    assert_eq!(file_state(&fixture.index_path()), before);
    assert_eq!(
        fixture.open().unwrap().history().unwrap(),
        fixture.history()
    );
}

#[test]
fn existing_cache_rejects_other_entry_hubs_and_rotated_entry_keys_without_rewriting() {
    let fixture = committed();
    let entry = fixture.peer.entry.identity;
    let wrong = Identity::new();
    let same_key_other_hub = PeerIdentity::new(HubId::new(), *entry.key()).unwrap();
    let before = file_state(&fixture.index_path());
    for identity in [wrong.identity, rotated(entry).identity, same_key_other_hub] {
        assert!(
            ObservationStore::open(
                PrivateDirectory::open(&fixture.path).unwrap(),
                identity,
                routing_limits()
            )
            .is_err()
        );
        assert_eq!(file_state(&fixture.index_path()), before);
    }
    assert_eq!(
        fixture.open().unwrap().history().unwrap(),
        fixture.history()
    );
}

#[test]
fn same_bytes_index_or_record_replacement_retires_the_active_owner() {
    for replace_index in [true, false] {
        let fixture = committed();
        let mut store = fixture.open().unwrap();
        let path = if replace_index {
            fixture.index_path()
        } else {
            fixture.record_path(fixture.peer.origin.identity.hub())
        };
        let before = file_state(&path);
        replace_same(&path, &fixture.sibling(DISPLACED));
        let replaced = file_state(&path);
        assert_eq!(replaced.0, before.0);
        assert_ne!(replaced.1, before.1);
        assert!(store.history().is_err());
        assert!(store.record_history(&fixture.history()).is_err());
        assert_eq!(file_state(&path), replaced);
    }
}

#[test]
fn root_replacement_poisoning_remains_sticky_after_the_original_directory_is_restored() {
    let fixture = committed();
    let mut store = fixture.open().unwrap();
    let displaced = fixture.sibling(DISPLACED);
    let replacement = fixture.sibling(REPLACEMENT);
    let before = file_state(&fixture.index_path());
    fs::rename(&fixture.path, &displaced).unwrap();
    PrivateDirectory::open_or_create_strict(&fixture.path).unwrap();
    assert!(store.history().is_err());
    fs::rename(&fixture.path, &replacement).unwrap();
    fs::rename(&displaced, &fixture.path).unwrap();
    assert_eq!(file_state(&fixture.index_path()), before);
    assert!(store.history().is_err());
    assert!(store.record_history(&fixture.history()).is_err());
    drop(store);
    assert_eq!(
        fixture.open().unwrap().history().unwrap(),
        fixture.history()
    );
}

#[test]
fn an_unindexed_valid_origin_and_an_unknown_file_are_neither_adopted_nor_changed() {
    let fixture = committed();
    let orphan = Identity::new();
    let mut value = document(&fixture.record_path(fixture.peer.origin.identity.hub()));
    value[CATALOGUE_KEY] = serde_json::to_value(empty_catalogue(&orphan)).unwrap();
    let orphan_path = fixture.record_path(orphan.identity.hub());
    write_new(&orphan_path, &serde_json::to_vec(&value).unwrap());
    let unknown_path = fixture.path.join(UNKNOWN);
    write_new(&unknown_path, INVALID_BYTES);
    let before = (file_state(&orphan_path), file_state(&unknown_path));
    for _ in 0..TWO_ORIGINS {
        let store = fixture.open().unwrap();
        assert_eq!(store.history().unwrap(), fixture.history());
        assert_eq!(
            (file_state(&orphan_path), file_state(&unknown_path)),
            before
        );
    }
}

#[test]
fn an_existing_unindexed_origin_blocks_upsert_without_being_overwritten() {
    let fixture = committed();
    let orphan = Identity::new();
    let orphan_path = fixture.record_path(orphan.identity.hub());
    write_new(&orphan_path, INVALID_BYTES);
    let before = file_state(&orphan_path);
    let mut store = fixture.open().unwrap();
    assert!(
        store
            .record_history(&fixture.with(vec![empty_catalogue(&orphan)]))
            .is_err()
    );
    assert_eq!(file_state(&orphan_path), before);
    drop(store);
    assert_eq!(
        fixture.open().unwrap().history().unwrap(),
        fixture.history()
    );
}
