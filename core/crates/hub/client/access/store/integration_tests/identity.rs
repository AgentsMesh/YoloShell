use crate::{constants::*, fixture::*, model::*};
use std::fs;
use yolo_hub_client_access_store::ClientAccessStore;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn copied_record_cannot_authorize_another_directory_even_for_the_same_hub() {
    let fixture = Fixture::new();
    let store = fixture.open(Some(configuration())).unwrap();
    let original = store.current().unwrap();
    let bytes = fixture.bytes();
    let path = fixture.sibling(COPIED_DIRECTORY);
    let directory = PrivateDirectory::open_or_create_strict(&path).unwrap();
    write_new(&path.join(RECORD_FILE), &bytes);
    assert!(ClientAccessStore::open(directory, hub(), None).is_err());
    assert_eq!(fs::read(path.join(RECORD_FILE)).unwrap(), bytes);
    assert_eq!(store.current().unwrap(), original);
}

#[test]
fn namespace_replacement_or_symlink_poisoning_survives_restoring_the_original_directory() {
    for symlink in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open(Some(configuration())).unwrap();
        let original = store.current().unwrap();
        let retired = fixture.sibling(RETIRED_DIRECTORY);
        fs::rename(&fixture.path, &retired).unwrap();
        if symlink {
            std::os::unix::fs::symlink(&retired, &fixture.path).unwrap();
        } else {
            PrivateDirectory::open_or_create_strict(&fixture.path).unwrap();
        }
        assert_poisoned(&mut store, &original);
        if symlink {
            fs::remove_file(&fixture.path).unwrap();
        } else {
            assert!(!fixture.record().exists());
            fs::rename(&fixture.path, fixture.sibling(REPLACEMENT_DIRECTORY)).unwrap();
        }
        fs::rename(&retired, &fixture.path).unwrap();
        assert_poisoned(&mut store, &original);
        drop(store);
        assert_eq!(fixture.open(None).unwrap().current().unwrap(), original);
    }
}

#[test]
fn byte_identical_record_replacement_is_not_adopted_and_restoration_cannot_clear_poison() {
    let fixture = Fixture::new();
    let mut store = fixture.open(Some(configuration())).unwrap();
    let original = store.current().unwrap();
    let identity = fixture.file_identity();
    let bytes = fixture.bytes();
    let retired = fixture.sibling(RETIRED_RECORD);
    fs::rename(fixture.record(), &retired).unwrap();
    write_new(&fixture.record(), &bytes);
    assert!(!fixture.file_identity().same_object(identity));
    assert!(store.replace(original.revision, next(&original)).is_err());
    assert_poisoned(&mut store, &original);
    assert_eq!(fixture.bytes(), bytes);
    fs::rename(fixture.record(), fixture.sibling(REPLACEMENT_RECORD)).unwrap();
    fs::rename(&retired, fixture.record()).unwrap();
    assert_poisoned(&mut store, &original);
    drop(store);
    assert_eq!(fixture.open(None).unwrap().current().unwrap(), original);
}
