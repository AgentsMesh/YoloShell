use super::{
    constants::*,
    fixture::{self, Fixture},
};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};
use uuid::Uuid;
use yolo_distribution_release_store::{CURRENT_FILE_NAME, MANIFEST_MODE, ReleaseStore};

#[test]
fn absent_or_staged_only_installations_are_observed_without_creating_any_entry() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary
        .path()
        .canonicalize()
        .unwrap()
        .join(INSTALL_DIRECTORY);
    assert!(ReleaseStore::open_existing(&root).unwrap().is_none());
    assert!(!root.exists());
    fixture::directory(&root);
    let before = snapshot(&root);
    assert!(ReleaseStore::open_existing(&root).unwrap().is_none());
    assert_eq!(snapshot(&root), before);
    fixture::write(&root.join(CURRENT_FILE_NAME), &[], MANIFEST_MODE);
    let before = snapshot(&root);
    assert!(ReleaseStore::open_existing(&root).is_err());
    assert_eq!(snapshot(&root), before);
}

#[test]
fn readonly_inspection_preserves_all_names_modes_and_object_metadata() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let (stage, descriptor) = fixture.stage(FIRST_FILL);
    let installed = store
        .publish(stage, &descriptor, None, Uuid::new_v4())
        .unwrap();
    let before = snapshot(&fixture.root);
    let reader = ReleaseStore::open_existing(&fixture.root).unwrap().unwrap();
    let observed = reader.inspect_existing().unwrap().unwrap();
    assert_eq!(observed.selection(), installed.selection());
    assert_eq!(observed.descriptor(), installed.descriptor());
    assert_eq!(observed.release_directory(), installed.release_directory());
    drop(reader);
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
fn readonly_open_does_not_normalize_an_unsafe_empty_directory() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary
        .path()
        .canonicalize()
        .unwrap()
        .join(INSTALL_DIRECTORY);
    fixture::directory(&root);
    fs::set_permissions(&root, fs::Permissions::from_mode(UNSAFE_MODE)).unwrap();
    let before = snapshot(&root);
    assert!(ReleaseStore::open_existing(&root).is_err());
    assert_eq!(snapshot(&root), before);
}

#[derive(Debug, Eq, PartialEq)]
struct Entry {
    path: PathBuf,
    device: u64,
    inode: u64,
    mode: u32,
    length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

fn snapshot(root: &Path) -> Vec<Entry> {
    let mut entries = Vec::new();
    visit(root, &mut entries);
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries
}
fn visit(path: &Path, entries: &mut Vec<Entry>) {
    let metadata = fs::symlink_metadata(path).unwrap();
    entries.push(Entry {
        path: path.into(),
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        length: metadata.len(),
        modified: (metadata.mtime(), metadata.mtime_nsec()),
        changed: (metadata.ctime(), metadata.ctime_nsec()),
    });
    if metadata.is_dir() {
        for child in fs::read_dir(path).unwrap() {
            visit(&child.unwrap().path(), entries);
        }
    }
}
