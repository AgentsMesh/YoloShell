use crate::{DirectoryHandle, TemporaryDirectory};
use std::fs;

const OWNER_PREFIX: &str = "yolo-resource-lifetime-test-";
const CHILD_NAME: &str = "child";
const USAGE_DESCRIPTION: &str = "unconfirmed external consumer";

#[test]
fn cloned_and_descendant_handles_retain_the_same_resource_owner() {
    let mut owner = TemporaryDirectory::create(OWNER_PREFIX).unwrap();
    owner.create_child(CHILD_NAME).unwrap();
    let root = owner.handle().clone();
    let child = DirectoryHandle::open_at(&root, CHILD_NAME).unwrap();
    let usage = child.retain_resource_use(USAGE_DESCRIPTION).unwrap();
    let path = owner.path();
    assert!(owner.cleanup().is_err());
    assert!(path.join(CHILD_NAME).is_dir());
    usage.confirm_released();
    owner.cleanup().unwrap();
    assert!(!path.exists());
    assert!(root.retain_resource_use(USAGE_DESCRIPTION).is_err());
}

#[test]
fn unconfirmed_use_remains_sticky_after_the_last_guard_drops() {
    let mut owner = TemporaryDirectory::create(OWNER_PREFIX).unwrap();
    let path = owner.path();
    drop(owner.retain_resource_use(USAGE_DESCRIPTION).unwrap());
    owner
        .retain_resource_use("later confirmed user")
        .unwrap()
        .confirm_released();
    let error = owner.cleanup().unwrap_err();
    assert!(format!("{error:#}").contains(USAGE_DESCRIPTION));
    drop(owner);
    assert!(path.is_dir());
    fs::remove_dir(path).unwrap();
}

#[test]
fn active_external_use_blocks_cleanup_even_before_its_guard_is_dropped() {
    let owner = TemporaryDirectory::create(OWNER_PREFIX).unwrap();
    let path = owner.path();
    let usage = owner.retain_resource_use(USAGE_DESCRIPTION).unwrap();
    drop(owner);
    assert!(path.is_dir());
    usage.confirm_released();
    fs::remove_dir(path).unwrap();
}

#[test]
fn nested_owner_retirement_cannot_retire_its_ancestor_or_hide_ancestor_uses() {
    let mut owner = TemporaryDirectory::create(OWNER_PREFIX).unwrap();
    let mut child = owner.create_child(CHILD_NAME).unwrap();
    let child_owner = crate::DirectoryResourceOwner::attach(&mut child);
    let usage = child.retain_resource_use(USAGE_DESCRIPTION).unwrap();
    assert!(child_owner.begin_retirement().is_err());
    assert!(owner.cleanup().is_err());
    usage.confirm_released();
    child_owner.begin_retirement().unwrap();
    assert!(child.retain_resource_use(USAGE_DESCRIPTION).is_err());
    owner
        .retain_resource_use(USAGE_DESCRIPTION)
        .unwrap()
        .confirm_released();
    owner.cleanup().unwrap();
}
