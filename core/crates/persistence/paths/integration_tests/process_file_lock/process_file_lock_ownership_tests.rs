use super::*;
use crate::paths::constants::PRIVATE_FILE_MODE;
use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

const ASSERT_BLOCKED_MODE: &str = "assert-exclusive-blocked";
const ASSERT_AVAILABLE_MODE: &str = "assert-exclusive-available";
const DESCRIPTOR_REUSE_MODE: &str = "exercise-descriptor-reuse";
const SOURCE_FILE_NAME: &str = "owned-source.lock";
const REUSED_FILE_NAME: &str = "reused-descriptor.lock";
const FLOCK_SUCCESS: libc::c_int = 0;
const FOREIGN_PROCESS_ID_INCREMENT: u32 = 1;

#[test]
fn token_owns_the_kernel_lock_after_the_source_file_is_dropped() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(SOURCE_FILE_NAME);
    create_file(&path);
    let file = open_file(&path);
    let identity = ObjectIdentity::from_metadata(&file.metadata().unwrap());
    let lock = ProcessFileLock::try_lock(&file, identity, Mode::Shared)
        .unwrap()
        .unwrap();

    drop(file);
    assert_external(&path, ASSERT_BLOCKED_MODE);
    drop(lock);
    assert_external(&path, ASSERT_AVAILABLE_MODE);
}

#[test]
fn shared_tokens_release_the_kernel_lease_only_after_the_last_drop() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(SOURCE_FILE_NAME);
    create_file(&path);
    let file = open_file(&path);
    let identity = ObjectIdentity::from_metadata(&file.metadata().unwrap());
    let first = ProcessFileLock::try_lock(&file, identity, Mode::Shared)
        .unwrap()
        .unwrap();
    let second = ProcessFileLock::try_lock(&file, identity, Mode::Shared)
        .unwrap()
        .unwrap();

    drop(first);
    assert_external(&path, ASSERT_BLOCKED_MODE);
    drop(second);
    assert_external(&path, ASSERT_AVAILABLE_MODE);
}

#[test]
fn dropping_a_token_never_unlocks_a_reused_descriptor() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(SOURCE_FILE_NAME);
    create_file(&path);
    assert!(
        super::kernel_tests::helper_command(DESCRIPTOR_REUSE_MODE, &path)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn foreign_process_token_drop_does_not_unlock_a_retained_kernel_lease() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(SOURCE_FILE_NAME);
    create_file(&path);
    let file = open_file(&path);
    let identity = ObjectIdentity::from_metadata(&file.metadata().unwrap());
    let descriptor = file.try_clone().unwrap();
    assert_eq!(
        unsafe { libc::flock(descriptor.as_raw_fd(), libc::LOCK_SH) },
        FLOCK_SUCCESS
    );
    drop(file);
    let foreign_process_id = std::process::id()
        .checked_add(FOREIGN_PROCESS_ID_INCREMENT)
        .unwrap();
    let lease = Arc::new(KernelLease {
        descriptor,
        locked: AtomicBool::new(true),
        process_id: foreign_process_id,
    });
    let retained = Arc::clone(&lease);
    let foreign = ProcessFileLock {
        lease: LockLease::Kernel(lease),
        key: identity,
        mode: Mode::Shared,
        process_id: foreign_process_id,
    };
    drop(foreign);
    assert_external(&path, ASSERT_BLOCKED_MODE);
    drop(retained);
    assert_external(&path, ASSERT_AVAILABLE_MODE);
}

pub(super) fn run_helper(mode: &str, path: &Path) -> bool {
    match mode {
        ASSERT_BLOCKED_MODE => assert_kernel_state(path, false),
        ASSERT_AVAILABLE_MODE => assert_kernel_state(path, true),
        DESCRIPTOR_REUSE_MODE => exercise_descriptor_reuse(path),
        _ => return false,
    }
    true
}

fn exercise_descriptor_reuse(path: &Path) {
    let reused_path = path.with_file_name(REUSED_FILE_NAME);
    create_file(&reused_path);
    let source = open_file(path);
    let source_descriptor = source.as_raw_fd();
    let identity = ObjectIdentity::from_metadata(&source.metadata().unwrap());
    let lock = ProcessFileLock::try_lock(&source, identity, Mode::Shared)
        .unwrap()
        .unwrap();
    drop(source);
    let unrelated = open_file(&reused_path);
    assert_eq!(unrelated.as_raw_fd(), source_descriptor);
    assert_eq!(
        unsafe { libc::flock(unrelated.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        FLOCK_SUCCESS
    );
    drop(lock);
    assert_external(&reused_path, ASSERT_BLOCKED_MODE);
    drop(unrelated);
    assert_external(&reused_path, ASSERT_AVAILABLE_MODE);
}

fn assert_kernel_state(path: &Path, expected_available: bool) {
    let file = open_file(path);
    let identity = ObjectIdentity::from_metadata(&file.metadata().unwrap());
    let available = ProcessFileLock::try_lock(&file, identity, Mode::Exclusive)
        .unwrap()
        .is_some();
    assert_eq!(available, expected_available);
}

fn assert_external(path: &Path, mode: &str) {
    assert!(
        super::kernel_tests::helper_command(mode, path)
            .status()
            .unwrap()
            .success()
    );
}

fn create_file(path: &Path) {
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
}

fn open_file(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap()
}
