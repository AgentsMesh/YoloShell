use crate::{DirectoryResourceUse, TemporaryDirectory};
use std::fs;
use std::os::unix::fs::PermissionsExt;

const PREFIX: &str = "yolo-temporary-fault-test-";
const SENTINEL: &str = "owned-evidence";
const BYTES: &[u8] = b"retain all bytes until resource absence is proven";
const READ_ONLY_MODE: u32 = 0o400;
const MODE_MASK: u32 = 0o777;
const INVALID_CHILD: &str = "unopened\0child";
const USAGE: &str = "container directory lease";
const RETIREMENT_FAILURE: &str = "injected failure before namespace retirement";
const MOVED_NAME: &str = "owned-but-moved";

fn evidence(owner: &TemporaryDirectory) {
    let file = owner.path().join(SENTINEL);
    fs::write(&file, BYTES).unwrap();
    fs::set_permissions(file, fs::Permissions::from_mode(READ_ONLY_MODE)).unwrap();
}
fn assert_preserved(path: &std::path::Path) {
    let file = path.join(SENTINEL);
    assert_eq!(fs::read(&file).unwrap(), BYTES);
    assert_eq!(
        fs::metadata(file).unwrap().permissions().mode() & MODE_MASK,
        READ_ONLY_MODE
    );
}

#[test]
fn failed_pre_retirement_barrier_preserves_bytes_modes_and_blocks_new_consumers() {
    let mut owner = TemporaryDirectory::create(PREFIX).unwrap();
    evidence(&owner);
    let root = owner.path();
    let error = owner
        .cleanup_with(|| anyhow::bail!(RETIREMENT_FAILURE))
        .unwrap_err();
    assert_eq!(error.to_string(), RETIREMENT_FAILURE);
    assert_eq!(owner.path(), root);
    assert_preserved(&root);
    assert!(owner.retain_resource_use(USAGE).is_err());
    owner.cleanup().unwrap();
    assert!(!root.exists());
}

#[test]
fn rejected_child_name_leaves_sticky_construction_uncertainty() {
    let mut owner = TemporaryDirectory::create(PREFIX).unwrap();
    evidence(&owner);
    let root = owner.path();
    assert!(owner.create_child(INVALID_CHILD).is_err());
    assert!(owner.cleanup().is_err());
    drop(owner);
    assert_preserved(&root);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_resource_set_acquisition_releases_all_earlier_confirmed_reservations() {
    let mut first = TemporaryDirectory::create(PREFIX).unwrap();
    let mut retired = TemporaryDirectory::create(PREFIX).unwrap();
    let stale = retired.handle().clone();
    retired.cleanup().unwrap();
    assert!(DirectoryResourceUse::retain([first.handle(), &stale], USAGE).is_err());
    let root = first.path();
    first.cleanup().unwrap();
    assert!(!root.exists());
}

#[test]
fn disappearing_name_before_retirement_does_not_mutate_moved_original() {
    let mut owner = TemporaryDirectory::create(PREFIX).unwrap();
    evidence(&owner);
    let root = owner.path();
    let saved = tempfile::tempdir().unwrap();
    let moved = saved.path().join(MOVED_NAME);
    let result = owner.cleanup_with(|| {
        fs::rename(&root, &moved)?;
        Ok(())
    });
    assert!(result.is_err());
    drop(owner);
    assert!(!root.exists());
    assert_preserved(&moved);
}
