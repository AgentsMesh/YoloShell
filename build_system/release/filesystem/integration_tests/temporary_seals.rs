use crate::constants::{TEMPORARY_DIRECTORY_MODE, TEMPORARY_NO_FLAGS};
use crate::{DirectoryHandle, TemporaryDirectory};

const OWNER_PREFIX: &str = "yolo-sealed-cleanup-test-";

#[test]
fn sealed_owner_unwinds_cleanly_after_capture_failure() {
    let mut owner = TemporaryDirectory::create(OWNER_PREFIX).unwrap();
    let path = owner.path();
    let result = (move || -> anyhow::Result<()> {
        owner.seal()?;
        anyhow::bail!("injected failure after sealing")
    })();
    assert!(result.is_err());
    assert!(!path.exists());
}

#[test]
fn moving_an_owner_preserves_its_seal_until_cleanup() {
    let mut owner = TemporaryDirectory::create(OWNER_PREFIX).unwrap();
    let path = owner.path();
    let handle = owner.handle().clone();
    let seal = owner.seal().unwrap();
    let mut transferred = owner;
    handle.verify_seal(&seal).unwrap();
    transferred.cleanup().unwrap();
    assert!(!path.exists());
}

#[test]
fn preexisting_root_replacement_keeps_both_objects_untouched() {
    let mut owner = TemporaryDirectory::create(OWNER_PREFIX).unwrap();
    let path = owner.path();
    let saved = tempfile::tempdir().unwrap();
    let moved = saved.path().join("moved");
    owner.seal().unwrap();
    owner.handle().set_flags(TEMPORARY_NO_FLAGS).unwrap();
    owner.handle().set_mode(TEMPORARY_DIRECTORY_MODE).unwrap();
    std::fs::rename(&path, &moved).unwrap();
    std::fs::create_dir(&path).unwrap();
    let foreign = DirectoryHandle::open(&path).unwrap();
    foreign.set_mode(TEMPORARY_DIRECTORY_MODE).unwrap();
    let seal = foreign.seal().unwrap();
    drop(owner);
    foreign.verify_seal(&seal).unwrap();
    std::fs::remove_dir(path).unwrap();
    crate::restore_owner_write(&moved).unwrap();
}
