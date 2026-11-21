use super::{fixture::*, *};
use std::fs;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn replaced_file_is_preserved_and_old_owner_permanently_retires() {
    let (_root, path, hub, mut store) = fixture();
    let request = update(hub);
    store.update(request.clone()).unwrap();
    let original = fs::read(path.join(LAYOUT_FILE)).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    yolo_paths::atomic_write_json(&path.join(LAYOUT_FILE), &value).unwrap();
    let replacement = fs::read(path.join(LAYOUT_FILE)).unwrap();
    assert!(store.get(request.address).is_err());
    assert!(store.update(request.clone()).is_err());
    assert!(store.get(request.address).is_err());
    assert_eq!(fs::read(path.join(LAYOUT_FILE)).unwrap(), replacement);
}

#[test]
fn renamed_namespace_and_deleted_record_never_recreate_state() {
    let (root, path, hub, mut store) = fixture();
    fs::rename(&path, root.path().join(RETIRED)).unwrap();
    PrivateDirectory::ensure(&path).unwrap();
    assert!(store.update(update(hub)).is_err());
    assert!(!path.join(LAYOUT_FILE).exists());
    let (_root, path, hub, mut store) = fixture();
    fs::remove_file(path.join(LAYOUT_FILE)).unwrap();
    assert!(store.update(update(hub)).is_err());
    assert!(!path.join(LAYOUT_FILE).exists());
}

#[test]
fn symlink_and_corrupt_record_cannot_redirect_or_silently_reset_layouts() {
    let (root, path, hub, mut store) = fixture();
    let foreign = root.path().join(REPLACEMENT);
    fs::write(&foreign, INVALID_BYTES).unwrap();
    fs::remove_file(path.join(LAYOUT_FILE)).unwrap();
    std::os::unix::fs::symlink(&foreign, path.join(LAYOUT_FILE)).unwrap();
    assert!(store.update(update(hub)).is_err());
    drop(store);
    assert!(LayoutStore::open(PrivateDirectory::open(&path).unwrap(), hub).is_err());
    assert_eq!(fs::read(&foreign).unwrap(), INVALID_BYTES);
    fs::remove_file(path.join(LAYOUT_FILE)).unwrap();
    fs::write(path.join(LAYOUT_FILE), INVALID_BYTES).unwrap();
    assert!(LayoutStore::open(PrivateDirectory::open(&path).unwrap(), hub).is_err());
    assert_eq!(fs::read(path.join(LAYOUT_FILE)).unwrap(), INVALID_BYTES);
}
