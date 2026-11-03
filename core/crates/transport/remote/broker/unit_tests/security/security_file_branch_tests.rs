use super::*;
use crate::remote_broker::constants::CONFIG_LABEL;
use std::fs::{self, File, FileTimes};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::time::{Duration, SystemTime};

const DEVICE_ENTRY: &str = "/dev/null";
const TEST_FILE: &str = "tested";
const PRIVATE_MODE: u32 = 0o600;
const READ_ONLY_MODE: u32 = 0o400;
const FILE_BYTES: &[u8] = b"secure";
const CTIME_TEST_FILE: &str = "ctime-tested";
const FIXED_MTIME_SECONDS: u64 = 1;
const CTIME_TICK: Duration = Duration::from_millis(1_100);

#[test]
fn stability_rejects_a_different_source_device() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(TEST_FILE);
    write_private(&path);
    let before = fs::metadata(&path).unwrap();
    let device = fs::metadata(DEVICE_ENTRY).unwrap();
    assert_ne!(before.dev(), device.dev());

    assert!(
        verify_secure_file_stable(&path, CONFIG_LABEL, &before, &device, FILE_BYTES.len()).is_err()
    );
}

#[test]
fn stability_rejects_same_inode_mtime_and_ctime_changes() {
    let root = tempfile::tempdir().unwrap();
    let mtime_path = root.path().join(TEST_FILE);
    write_private(&mtime_path);
    let file = File::open(&mtime_path).unwrap();
    let before_mtime = file.metadata().unwrap();
    file.set_times(
        FileTimes::new()
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(FIXED_MTIME_SECONDS)),
    )
    .unwrap();
    let after_mtime = file.metadata().unwrap();
    assert_ne!(before_mtime.mtime(), after_mtime.mtime());
    assert!(
        verify_secure_file_stable(
            &mtime_path,
            CONFIG_LABEL,
            &before_mtime,
            &after_mtime,
            FILE_BYTES.len(),
        )
        .is_err()
    );

    let ctime_path = root.path().join(CTIME_TEST_FILE);
    write_private(&ctime_path);
    let before_ctime = fs::metadata(&ctime_path).unwrap();
    std::thread::sleep(CTIME_TICK);
    fs::set_permissions(&ctime_path, fs::Permissions::from_mode(READ_ONLY_MODE)).unwrap();
    let after_ctime = fs::metadata(&ctime_path).unwrap();
    assert_eq!(before_ctime.mtime(), after_ctime.mtime());
    assert_ne!(before_ctime.ctime(), after_ctime.ctime());
    assert!(
        verify_secure_file_stable(
            &ctime_path,
            CONFIG_LABEL,
            &before_ctime,
            &after_ctime,
            FILE_BYTES.len(),
        )
        .is_err()
    );
}

#[test]
fn stability_rejects_a_republished_path_on_another_device() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(TEST_FILE);
    write_private(&path);
    let stable = fs::metadata(&path).unwrap();
    assert_ne!(stable.dev(), fs::metadata(DEVICE_ENTRY).unwrap().dev());

    assert!(
        verify_secure_file_stable(
            Path::new(DEVICE_ENTRY),
            CONFIG_LABEL,
            &stable,
            &stable,
            FILE_BYTES.len(),
        )
        .is_err()
    );
}

fn write_private(path: &Path) {
    fs::write(path, FILE_BYTES).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_MODE)).unwrap();
}
