use super::fixtures::{constants::*, *};
use crate::constants::*;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn namespace_replacement_poisoning_survives_putting_the_original_directory_back() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let request = change(&store.current().unwrap());
    let retired = fixture.sibling(RETIRED_DIR);
    std::fs::rename(&fixture.path, &retired).unwrap();
    PrivateDirectory::ensure(&fixture.path).unwrap();
    assert!(store.current().is_err());
    assert!(store.apply(request).is_err());
    std::fs::remove_dir(&fixture.path).unwrap();
    std::fs::rename(&retired, &fixture.path).unwrap();
    assert!(store.current().is_err());
}

#[test]
fn replaced_lock_record_revokes_ownership_before_configuration_read_or_write() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let request = change(&store.current().unwrap());
    let bytes = fixture.bytes();
    std::fs::remove_file(fixture.path.join(LOCK_FILE)).unwrap();
    fixture
        .namespace()
        .write_json(&fixture.path.join(LOCK_FILE), &UNKNOWN_VALUE)
        .unwrap();
    assert!(store.current().is_err());
    assert!(store.apply(request).is_err());
    assert_eq!(fixture.bytes(), bytes);
}

#[test]
fn missing_symlinked_or_world_readable_records_cannot_be_used_or_recreated_in_place() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let request = change(&store.current().unwrap());
    fixture
        .namespace()
        .remove_file(&fixture.path.join(RECORD_FILE))
        .unwrap();
    assert!(store.current().is_err());
    assert!(store.apply(request).is_err());
    assert!(!fixture.path.join(RECORD_FILE).exists());
    drop(store);
    std::os::unix::fs::symlink(fixture.sibling(CLONE_DIR), fixture.path.join(RECORD_FILE)).unwrap();
    assert!(fixture.open(None).is_err());
    let fixture = Fixture::new();
    let store = fixture.open(None).unwrap();
    drop(store);
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(
        fixture.path.join(RECORD_FILE),
        std::fs::Permissions::from_mode(WORLD_READABLE),
    )
    .unwrap();
    assert!(fixture.open(None).is_err());
}
