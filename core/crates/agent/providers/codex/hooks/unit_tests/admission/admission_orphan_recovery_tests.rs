use super::admission_orphan_test_support::*;
use super::admission_test_support::Fixture;
use std::os::unix::fs::{PermissionsExt, symlink};

const RECOVERED_INVOCATION: u128 = 0x74000000000000000000000000000001;
const MISSING_LOCK_INVOCATION: u128 = 0x74000000000000000000000000000002;
const BUSY_LOCK_INVOCATION: u128 = 0x74000000000000000000000000000003;
const LOOKALIKE_INVOCATION: u128 = 0x74000000000000000000000000000004;
const SYMLINK_INVOCATION: u128 = 0x74000000000000000000000000000005;
const UNSAFE_MODE_INVOCATION: u128 = 0x74000000000000000000000000000006;
const OVERSIZED_INVOCATION: u128 = 0x74000000000000000000000000000007;
const HARD_LINK_INVOCATION: u128 = 0x74000000000000000000000000000008;
const INTERRUPTED_RECLAIM_INVOCATION: u128 = 0x74000000000000000000000000000009;
const LOOKALIKE_NAME: &str = ".yolo-state-not-the-admission-stage.tmp";
const OUTSIDE_NAME: &str = "outside-stage-target";
const UNSAFE_FILE_MODE: u32 = 0o644;
const OVERSIZE_INCREMENT_BYTES: u64 = 1;
const INTERRUPTED_RECLAIM: &str = "simulated interrupted staging reclaim";

#[test]
fn scan_reclaims_exact_pre_commit_admission_staging() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, RECOVERED_INVOCATION);
    let mut expected = [
        staged
            .artifacts
            .retirement_writer_lock_path()
            .file_name()
            .unwrap()
            .to_owned(),
        staged
            .artifacts
            .intake_directory()
            .file_name()
            .unwrap()
            .to_owned(),
    ];
    expected.sort();

    assert!(scan(&fixture).unwrap().is_empty());
    assert!(!staged.staging_path.exists());
    assert!(!staged.target_path.exists());
    assert_eq!(entry_names(staged.artifacts.directory()), expected);
}

#[test]
fn scan_preserves_staging_without_its_writer_lock() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, MISSING_LOCK_INVOCATION);
    std::fs::remove_file(staged.artifacts.retirement_writer_lock_path()).unwrap();

    assert!(scan(&fixture).is_err());
    assert_eq!(std::fs::read(staged.staging_path).unwrap(), STAGING_BYTES);
}

#[test]
fn scan_preserves_staging_while_its_writer_lock_is_held() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, BUSY_LOCK_INVOCATION);
    let writer = acquire_writer(&staged);

    assert!(scan(&fixture).is_err());
    assert_eq!(std::fs::read(&staged.staging_path).unwrap(), STAGING_BYTES);
    drop(writer);
    assert!(scan(&fixture).unwrap().is_empty());
}

#[test]
fn scan_rejects_noncanonical_staging_lookalikes() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, LOOKALIKE_INVOCATION);
    std::fs::rename(
        &staged.staging_path,
        staged.staging_path.with_file_name(LOOKALIKE_NAME),
    )
    .unwrap();

    assert!(scan(&fixture).is_err());
    assert!(staged.staging_path.with_file_name(LOOKALIKE_NAME).exists());
}

#[test]
fn scan_never_follows_an_exact_staging_symlink() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, SYMLINK_INVOCATION);
    std::fs::remove_file(&staged.staging_path).unwrap();
    let outside = staged
        .artifacts
        .directory()
        .parent()
        .unwrap()
        .join(OUTSIDE_NAME);
    write_private(&outside, REPLACEMENT_BYTES);
    symlink(&outside, &staged.staging_path).unwrap();

    assert!(scan(&fixture).is_err());
    assert!(
        std::fs::symlink_metadata(&staged.staging_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(std::fs::read(outside).unwrap(), REPLACEMENT_BYTES);
}

#[test]
fn scan_preserves_an_exact_staging_file_with_unsafe_mode() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, UNSAFE_MODE_INVOCATION);
    std::fs::set_permissions(
        &staged.staging_path,
        std::fs::Permissions::from_mode(UNSAFE_FILE_MODE),
    )
    .unwrap();

    assert!(scan(&fixture).is_err());
    assert_eq!(std::fs::read(staged.staging_path).unwrap(), STAGING_BYTES);
}

#[test]
fn scan_preserves_an_oversized_exact_staging_file() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, OVERSIZED_INVOCATION);
    let oversized = crate::paths::constants::MAX_DURABLE_JSON_BYTES + OVERSIZE_INCREMENT_BYTES;
    std::fs::OpenOptions::new()
        .write(true)
        .open(&staged.staging_path)
        .unwrap()
        .set_len(oversized)
        .unwrap();

    assert!(scan(&fixture).is_err());
    assert_eq!(
        std::fs::metadata(staged.staging_path).unwrap().len(),
        oversized
    );
}

#[test]
fn scan_preserves_a_hard_linked_exact_staging_file() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, HARD_LINK_INVOCATION);
    let outside = staged
        .artifacts
        .directory()
        .parent()
        .unwrap()
        .join(OUTSIDE_NAME);
    std::fs::hard_link(&staged.staging_path, &outside).unwrap();

    assert!(scan(&fixture).is_err());
    assert_eq!(std::fs::read(staged.staging_path).unwrap(), STAGING_BYTES);
    assert_eq!(std::fs::read(outside).unwrap(), STAGING_BYTES);
}

#[test]
fn scan_resumes_an_interrupted_staging_isolation_reclaim() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, INTERRUPTED_RECLAIM_INVOCATION);
    let staging = crate::paths::private_file::PrivateFileEntry::open(&staged.staging_path).unwrap();
    let isolation = staging.unlink_isolation_entry().unwrap();
    let _hook = crate::paths::test_support::install_before_unlink_hook(&staging, || {
        anyhow::bail!(INTERRUPTED_RECLAIM)
    });

    assert!(scan(&fixture).is_err());
    assert!(!staged.staging_path.exists());
    assert!(isolation.path().exists());
    assert_eq!(
        std::fs::metadata(isolation.path()).unwrap().len(),
        u64::default()
    );

    assert!(scan(&fixture).unwrap().is_empty());
    assert!(!isolation.path().exists());
    assert!(!staged.target_path.exists());
}
