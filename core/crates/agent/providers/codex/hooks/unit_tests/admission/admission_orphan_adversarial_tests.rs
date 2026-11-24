use super::admission_orphan_test_support::*;
use super::admission_test_support::Fixture;

const STAGE_SWAP_INVOCATION: u128 = 0x75000000000000000000000000000001;
const TARGET_INSERT_INVOCATION: u128 = 0x75000000000000000000000000000002;
const LOCK_SWAP_INVOCATION: u128 = 0x75000000000000000000000000000003;
const DIRECTORY_SWAP_INVOCATION: u128 = 0x75000000000000000000000000000004;
const FORGED_ISOLATION_INVOCATION: u128 = 0x75000000000000000000000000000005;
const SAVED_STAGE_NAME: &str = "saved-admission-stage";
const SAVED_LOCK_NAME: &str = "saved-writer-lock";
const DISPLACED_DIRECTORY_NAME: &str = "displaced-invocation";

#[test]
fn staging_replacement_after_inode_lock_is_preserved() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, STAGE_SWAP_INVOCATION);
    let saved = staged.staging_path.with_file_name(SAVED_STAGE_NAME);
    let staging_for_hook = staged.staging_path.clone();
    let saved_for_hook = saved.clone();
    super::admission_orphan::set_before_stage_reclaim_hook(move || {
        std::fs::rename(&staging_for_hook, &saved_for_hook).unwrap();
        write_private(&staging_for_hook, REPLACEMENT_BYTES);
    });

    assert!(scan(&fixture).is_err());
    assert_eq!(
        std::fs::read(staged.staging_path).unwrap(),
        REPLACEMENT_BYTES
    );
    assert_eq!(std::fs::read(saved).unwrap(), STAGING_BYTES);
}

#[test]
fn target_insertion_before_reclaim_preserves_the_stage() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, TARGET_INSERT_INVOCATION);
    let target_for_hook = staged.target_path.clone();
    super::admission_orphan::set_before_stage_reclaim_hook(move || {
        write_private(&target_for_hook, REPLACEMENT_BYTES);
    });

    assert!(scan(&fixture).is_err());
    assert_eq!(std::fs::read(staged.staging_path).unwrap(), STAGING_BYTES);
    assert_eq!(
        std::fs::read(staged.target_path).unwrap(),
        REPLACEMENT_BYTES
    );
}

#[test]
fn writer_lock_replacement_before_reclaim_preserves_the_stage() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, LOCK_SWAP_INVOCATION);
    let lock = staged.artifacts.retirement_writer_lock_path();
    let saved = lock.with_file_name(SAVED_LOCK_NAME);
    let lock_for_hook = lock.clone();
    let saved_for_hook = saved.clone();
    super::admission_orphan::set_before_stage_reclaim_hook(move || {
        std::fs::rename(&lock_for_hook, &saved_for_hook).unwrap();
        write_private(&lock_for_hook, &[]);
    });

    assert!(scan(&fixture).is_err());
    assert_eq!(std::fs::read(staged.staging_path).unwrap(), STAGING_BYTES);
    assert!(lock.exists());
    assert!(saved.exists());
}

#[test]
fn invocation_directory_replacement_before_reclaim_preserves_the_stage() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, DIRECTORY_SWAP_INVOCATION);
    let directory = staged.artifacts.directory().to_path_buf();
    let displaced = directory.with_file_name(DISPLACED_DIRECTORY_NAME);
    let directory_for_hook = directory.clone();
    let displaced_for_hook = displaced.clone();
    super::admission_orphan::set_before_stage_reclaim_hook(move || {
        std::fs::rename(&directory_for_hook, &displaced_for_hook).unwrap();
        crate::paths::directory::PrivateDirectory::ensure(&directory_for_hook).unwrap();
    });

    assert!(scan(&fixture).is_err());
    assert!(
        displaced
            .join(staged.staging_path.file_name().unwrap())
            .exists()
    );
    std::fs::remove_dir(&directory).unwrap();
    std::fs::rename(displaced, directory).unwrap();
}

#[test]
fn nonempty_forged_staging_isolation_is_preserved() {
    let fixture = Fixture::new();
    let staged = stage(&fixture, FORGED_ISOLATION_INVOCATION);
    let staging = crate::paths::private_file::PrivateFileEntry::open(&staged.staging_path).unwrap();
    let isolation = staging.unlink_isolation_entry().unwrap();
    std::fs::rename(&staged.staging_path, isolation.path()).unwrap();

    assert!(scan(&fixture).is_err());
    assert_eq!(std::fs::read(isolation.path()).unwrap(), STAGING_BYTES);
    assert!(!staged.target_path.exists());
}
