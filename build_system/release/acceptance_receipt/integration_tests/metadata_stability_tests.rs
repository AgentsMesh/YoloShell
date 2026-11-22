use crate::metadata::stable;
use std::fs::{self, File, FileTimes};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::time::{Duration, SystemTime};

const RECEIPT: &str = "receipt.json";
const SECOND_RECEIPT: &str = "other.json";
const READ_ONLY_MODE: u32 = 0o400;
const CONTENT: &[u8] = b"receipt";
const DEVICE_PATH: &str = "/dev/null";
const REFERENCE_SECONDS: u64 = 10_000;
const ONE_SECOND: u64 = 1;
const ONE_NANOSECOND: u64 = 1;

#[test]
fn metadata_identity_compares_the_device_and_inode_before_accepting_a_receipt() {
    let root = tempfile::tempdir().unwrap();
    let original = File::create(root.path().join(RECEIPT))
        .unwrap()
        .metadata()
        .unwrap();
    let other = File::create(root.path().join(SECOND_RECEIPT))
        .unwrap()
        .metadata()
        .unwrap();
    assert_eq!(original.dev(), other.dev());
    assert_ne!(original.ino(), other.ino());
    assert!(stable(&original, &original));
    assert!(!stable(&original, &other));
    let device = fs::metadata(DEVICE_PATH).unwrap();
    assert_ne!(original.dev(), device.dev());
    assert!(!stable(&original, &device));
}

#[test]
fn metadata_stability_rejects_permission_link_and_size_changes_independently() {
    let mutations: &[fn(&std::path::Path, &std::path::Path)] = &[
        |path, _| fs::set_permissions(path, fs::Permissions::from_mode(READ_ONLY_MODE)).unwrap(),
        |path, extra| fs::hard_link(path, extra).unwrap(),
        |path, _| fs::write(path, CONTENT).unwrap(),
    ];
    for mutate in mutations {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(RECEIPT);
        let file = File::create(&path).unwrap();
        let before = file.metadata().unwrap();
        mutate(&path, &root.path().join(SECOND_RECEIPT));
        assert!(!stable(&before, &file.metadata().unwrap()));
    }
}

#[test]
fn both_second_and_subsecond_modification_changes_invalidate_receipt_stability() {
    let reference = SystemTime::UNIX_EPOCH + Duration::from_secs(REFERENCE_SECONDS);
    for shift in [
        Duration::from_secs(ONE_SECOND),
        Duration::from_nanos(ONE_NANOSECOND),
    ] {
        let file = tempfile::tempfile().unwrap();
        file.set_times(FileTimes::new().set_modified(reference))
            .unwrap();
        let before = file.metadata().unwrap();
        file.set_times(FileTimes::new().set_modified(reference + shift))
            .unwrap();
        assert!(!stable(&before, &file.metadata().unwrap()));
    }
}

#[test]
fn restoring_original_permissions_cannot_erase_the_change_timestamp() {
    let file = tempfile::tempfile().unwrap();
    let before = file.metadata().unwrap();
    file.set_permissions(fs::Permissions::from_mode(READ_ONLY_MODE))
        .unwrap();
    file.set_permissions(before.permissions()).unwrap();
    let after = file.metadata().unwrap();
    assert_eq!(before.mode(), after.mode());
    assert_eq!(before.mtime(), after.mtime());
    assert_eq!(before.mtime_nsec(), after.mtime_nsec());
    assert!(!stable(&before, &after));
}
