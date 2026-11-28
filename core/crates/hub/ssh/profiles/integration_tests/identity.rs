use super::{
    constants::*,
    fixture::{Fixture, write_new},
};
use std::fs;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_ssh_profile_protocol_test_support::{change, configuration, identity, profile};
use yolo_hub_ssh_profiles::SshProfileStore;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn copied_record_is_bound_to_original_directory_and_full_hub_identity() {
    let fixture = Fixture::new();
    let store = fixture.open(Some(configuration())).unwrap();
    let current = store.current().unwrap();
    drop(store);
    let clone = fixture.sibling(CLONE);
    let directory = PrivateDirectory::open_or_create_strict(&clone).unwrap();
    write_new(&clone.join(RECORD_FILE), &fixture.bytes());
    assert!(SshProfileStore::open(directory, fixture.hub, None).is_err());
    for other in [
        identity(),
        PeerIdentity::new(fixture.hub.hub(), *identity().key()).unwrap(),
    ] {
        assert!(
            SshProfileStore::open(PrivateDirectory::open(&fixture.path).unwrap(), other, None)
                .is_err()
        );
    }
    assert_eq!(fixture.open(None).unwrap().current().unwrap(), current);
}

#[test]
fn same_json_inode_replacement_is_not_adopted_and_restoring_original_cannot_clear_latch() {
    let fixture = Fixture::new();
    let mut store = fixture.open(Some(configuration())).unwrap();
    let current = store.current().unwrap();
    let bytes = fixture.bytes();
    let displaced = fixture.sibling(DISPLACED);
    fs::rename(fixture.record(), &displaced).unwrap();
    write_new(&fixture.record(), &bytes);
    assert!(store.current().is_err());
    assert!(store.resolve(&profile(0).profile).is_err());
    assert!(store.apply(change(&current, configuration())).is_err());
    assert_eq!(fixture.bytes(), bytes);
    fs::rename(fixture.record(), fixture.sibling(REPLACEMENT)).unwrap();
    fs::rename(&displaced, fixture.record()).unwrap();
    assert!(store.verify().is_err());
    drop(store);
    assert_eq!(fixture.open(None).unwrap().current().unwrap(), current);
}

#[test]
fn root_replacement_is_rejected_before_any_update_and_remains_latched_after_restore() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let current = store.current().unwrap();
    let displaced = fixture.sibling(DISPLACED);
    fs::rename(&fixture.path, &displaced).unwrap();
    PrivateDirectory::open_or_create_strict(&fixture.path).unwrap();
    assert!(store.verify().is_err());
    assert!(store.apply(change(&current, configuration())).is_err());
    assert!(!fixture.record().exists());
    fs::rename(&fixture.path, fixture.sibling(REPLACEMENT)).unwrap();
    fs::rename(&displaced, &fixture.path).unwrap();
    assert!(store.current().is_err());
}

#[test]
fn missing_or_corrupt_records_are_not_recreated_by_the_retained_owner() {
    for missing in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open(None).unwrap();
        let current = store.current().unwrap();
        if missing {
            fs::remove_file(fixture.record()).unwrap();
        } else {
            fs::write(fixture.record(), INVALID_BYTES).unwrap();
        }
        assert!(store.current().is_err());
        assert!(store.apply(change(&current, configuration())).is_err());
        if missing {
            assert!(!fixture.record().exists());
        } else {
            assert_eq!(fixture.bytes(), INVALID_BYTES);
        }
    }
}

#[test]
fn unversioned_record_and_foreign_directory_proof_cannot_be_adopted_on_reopen() {
    for remove_schema in [false, true] {
        let fixture = Fixture::new();
        drop(fixture.open(None).unwrap());
        let mut record: serde_json::Value = serde_json::from_slice(&fixture.bytes()).unwrap();
        if remove_schema {
            record.as_object_mut().unwrap().remove(SCHEMA_KEY);
        } else {
            let foreign = PrivateDirectory::ensure(&fixture.sibling(CLONE)).unwrap();
            record[DIRECTORY_KEY] =
                serde_json::to_value(foreign.durable_identity().unwrap()).unwrap();
        }
        fs::write(fixture.record(), serde_json::to_vec(&record).unwrap()).unwrap();
        let before = fixture.bytes();
        assert!(fixture.open(None).is_err());
        assert_eq!(fixture.bytes(), before);
    }
}
