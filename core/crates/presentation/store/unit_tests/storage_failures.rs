use super::{fixture::*, *};
use crate::{constants::LOCK_FILE, record::Record};
use std::fs;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn replaced_retained_directory_is_rejected_before_open_can_create_a_record() {
    let (root, path, hub, store) = fixture();
    drop(store);
    let retained = PrivateDirectory::open(&path).unwrap();
    let moved = root.path().join(RETIRED);
    fs::rename(&path, &moved).unwrap();
    PrivateDirectory::ensure(&path).unwrap();
    assert!(LayoutStore::open(retained, hub).is_err());
    assert!(!path.join(LAYOUT_FILE).exists());
    assert!(moved.join(LAYOUT_FILE).exists());
}

#[test]
fn unsafe_lock_and_record_entries_never_become_a_new_empty_store() {
    for locked in [true, false] {
        let (_root, path, hub, store) = fixture();
        drop(store);
        let entry = path.join(if locked { LOCK_FILE } else { LAYOUT_FILE });
        if entry.exists() {
            fs::remove_file(&entry).unwrap();
        }
        fs::create_dir(&entry).unwrap();
        assert!(LayoutStore::open(PrivateDirectory::open(&path).unwrap(), hub).is_err());
        assert!(entry.is_dir());
    }
}

#[test]
fn changed_record_content_and_lock_identity_retire_the_existing_owner() {
    let (_root, path, hub, mut store) = fixture();
    let request = update(hub);
    store.update(request.clone()).unwrap();
    let mut replacement: Record = yolo_paths::read_json(&path.join(LAYOUT_FILE)).unwrap();
    replacement.documents[0].tabs[0].title = NEXT_TITLE.into();
    yolo_paths::atomic_write_json(&path.join(LAYOUT_FILE), &replacement).unwrap();
    assert!(store.list().is_err());
    assert!(store.update(request).is_err());
    assert_eq!(
        yolo_paths::read_json::<Record>(&path.join(LAYOUT_FILE)).unwrap(),
        replacement
    );

    let (_root, path, hub, mut store) = fixture();
    let before = fs::read(path.join(LAYOUT_FILE)).unwrap();
    fs::remove_file(path.join(LOCK_FILE)).unwrap();
    assert!(store.update(update(hub)).is_err());
    assert_eq!(fs::read(path.join(LAYOUT_FILE)).unwrap(), before);
    assert!(store.list().is_err());
}

#[test]
fn malformed_and_foreign_records_fail_reopen_without_rewriting_bytes() {
    let (_root, path, hub, store) = fixture();
    drop(store);
    fs::write(path.join(LAYOUT_FILE), INVALID_BYTES).unwrap();
    assert!(LayoutStore::open(PrivateDirectory::open(&path).unwrap(), hub).is_err());
    assert_eq!(fs::read(path.join(LAYOUT_FILE)).unwrap(), INVALID_BYTES);
    let foreign = Record::empty(yolo_hub_identity::HubId::new());
    yolo_paths::atomic_write_json(&path.join(LAYOUT_FILE), &foreign).unwrap();
    let before = fs::read(path.join(LAYOUT_FILE)).unwrap();
    assert!(LayoutStore::open(PrivateDirectory::open(&path).unwrap(), hub).is_err());
    assert_eq!(fs::read(path.join(LAYOUT_FILE)).unwrap(), before);
}
