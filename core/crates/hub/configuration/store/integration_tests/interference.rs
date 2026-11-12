use crate::{constants::*, records, support::Fixture};
use yolo_hub_configuration_store::ConfigurationStore;
use yolo_paths::{directory::PrivateDirectory, namespace_lease::NamespaceLease};

#[test]
fn exact_content_replacement_after_check_is_not_permission_to_overwrite_it() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None);
    let current = store.current().unwrap();
    let change = records::change(&current);
    store.check(&change).unwrap();
    let previous = fixture.read();
    fixture.write(&previous);
    assert!(store.apply(change).is_err());
    assert!(store.current().is_err());
    assert_eq!(fixture.read(), previous);
    drop(store);
    assert_eq!(fixture.open(None).current().unwrap(), current);
}

#[test]
fn directory_replacement_and_copied_record_cannot_be_adopted_by_old_owner() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None);
    let current = store.current().unwrap();
    let copy = fixture
        .root
        .path()
        .canonicalize()
        .unwrap()
        .join(CLONE_DIRECTORY);
    let directory = PrivateDirectory::ensure(&copy).unwrap();
    let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)]).unwrap();
    namespace
        .write_json(&copy.join(RECORD_FILE), &fixture.read())
        .unwrap();
    assert!(
        ConfigurationStore::open(
            PrivateDirectory::open(&copy).unwrap(),
            fixture.hub.peer_identity(),
            None
        )
        .is_err()
    );
    let moved = fixture
        .root
        .path()
        .canonicalize()
        .unwrap()
        .join(MOVED_DIRECTORY);
    std::fs::rename(&fixture.path, &moved).unwrap();
    PrivateDirectory::ensure(&fixture.path).unwrap();
    assert!(store.current().is_err());
    assert!(store.apply(records::change(&current)).is_err());
    assert!(!fixture.path.join(RECORD_FILE).exists());
}

#[test]
fn replaced_singleton_lock_poisoned_owner_cannot_write_or_restore_authority() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None);
    let current = store.current().unwrap();
    let previous = fixture.read();
    std::fs::remove_file(fixture.path.join(LOCK_FILE)).unwrap();
    fixture
        .namespace()
        .write_json(&fixture.path.join(LOCK_FILE), &())
        .unwrap();
    assert!(store.apply(records::change(&current)).is_err());
    assert!(store.current().is_err());
    assert_eq!(fixture.read(), previous);
}

#[test]
fn missing_or_symlinked_record_is_not_recreated_and_symlink_target_is_untouched() {
    for symlink in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open(None);
        let current = store.current().unwrap();
        std::fs::remove_file(fixture.path.join(RECORD_FILE)).unwrap();
        let target = fixture.root.path().join(CLONE_DIRECTORY);
        if symlink {
            std::fs::write(&target, INVALID_DOCUMENT).unwrap();
            std::os::unix::fs::symlink(&target, fixture.path.join(RECORD_FILE)).unwrap();
        }
        assert!(store.current().is_err());
        assert!(store.apply(records::change(&current)).is_err());
        if symlink {
            assert_eq!(std::fs::read(&target).unwrap(), INVALID_DOCUMENT);
        } else {
            assert!(!fixture.path.join(RECORD_FILE).exists());
        }
    }
}
