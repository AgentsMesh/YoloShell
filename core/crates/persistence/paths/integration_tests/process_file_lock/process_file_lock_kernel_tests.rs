use super::*;
use crate::paths::constants::{
    PRIVATE_FILE_MODE, PROCESS_FILE_LOCK_TEST_HELPER_FILTER,
    PROCESS_FILE_LOCK_TEST_NOCAPTURE_ARGUMENT, PROCESS_FILE_LOCK_TEST_SIGNAL_BYTES,
};
use crate::paths::identity::ObjectIdentity;
use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const HELPER_MODE_ENV: &str = "YOLO_PROCESS_LOCK_HELPER_MODE";
const HELPER_PATH_ENV: &str = "YOLO_PROCESS_LOCK_HELPER_PATH";
const HELPER_READY_ENV: &str = "YOLO_PROCESS_LOCK_HELPER_READY";
const HELPER_RELEASE_ENV: &str = "YOLO_PROCESS_LOCK_HELPER_RELEASE";
const HOLD_SHARED_MODE: &str = "hold-shared";
const KERNEL_ERROR_MODE: &str = "kernel-error";
const LOCK_FILE_NAME: &str = "kernel.lock";
const READY_FILE_NAME: &str = "holder.ready";
const RELEASE_FILE_NAME: &str = "holder.release";
const UNEXPECTED_HELPER_MODE: &str = "unexpected process lock helper mode";
const HELPER_RELEASE_TIMEOUT: &str = "helper release signal timed out";
const HOLDER_EXITED_EARLY: &str = "lock holder exited early";
const HOLDER_READINESS_TIMEOUT: &str = "lock holder readiness timed out";
const HELPER_TIMEOUT: Duration = Duration::from_secs(5);
const HELPER_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[test]
fn direct_exclusive_lock_waits_for_an_external_shared_holder() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(LOCK_FILE_NAME);
    create_lock_file(&path);
    let file = open_lock_file(&path);
    let identity = ObjectIdentity::from_metadata(&file.metadata().unwrap());
    let ready = temporary.path().join(READY_FILE_NAME);
    let release = temporary.path().join(RELEASE_FILE_NAME);
    let mut holder = crate::process::spawn_waitable_test_child(
        helper_command(HOLD_SHARED_MODE, &path)
            .env(HELPER_READY_ENV, &ready)
            .env(HELPER_RELEASE_ENV, &release),
    );
    wait_for_holder(&ready, &mut holder);
    assert!(
        ProcessFileLock::try_lock(&file, identity, Mode::Exclusive)
            .unwrap()
            .is_none()
    );
    fs::write(&release, PROCESS_FILE_LOCK_TEST_SIGNAL_BYTES).unwrap();
    assert!(holder.wait().unwrap().success());
    assert!(
        ProcessFileLock::try_lock(&file, identity, Mode::Exclusive)
            .unwrap()
            .is_some()
    );
}

#[test]
fn logical_conflicts_short_circuit_both_kernel_lock_entry_points() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(LOCK_FILE_NAME);
    create_lock_file(&path);
    let file = open_lock_file(&path);
    let identity = ObjectIdentity::from_metadata(&file.metadata().unwrap());
    let owner = ProcessFileLock::try_lock(&file, identity, Mode::Exclusive)
        .unwrap()
        .unwrap();

    assert!(
        ProcessFileLock::try_lock(&file, identity, Mode::Shared)
            .unwrap()
            .is_none()
    );
    assert!(ProcessFileLock::lock_shared(&file, identity).is_err());
    drop(owner);
}

#[test]
#[cfg(target_os = "macos")]
fn kernel_lock_errors_release_logical_ownership() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(LOCK_FILE_NAME);
    create_lock_file(&path);
    assert!(
        helper_command(KERNEL_ERROR_MODE, &path)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn kernel_lock_subprocess_helper() {
    let Some(mode) = std::env::var_os(HELPER_MODE_ENV) else {
        return;
    };
    let path = PathBuf::from(std::env::var_os(HELPER_PATH_ENV).unwrap());
    let mode = mode.to_str().unwrap();
    if super::ownership_tests::run_helper(mode, &path) {
        return;
    }
    let file = open_lock_file(&path);
    let identity = ObjectIdentity::from_metadata(&file.metadata().unwrap());
    match mode {
        HOLD_SHARED_MODE => hold_shared(file, identity),
        KERNEL_ERROR_MODE => exercise_kernel_errors(file, identity),
        unexpected => panic!("{UNEXPECTED_HELPER_MODE}: {unexpected}"),
    }
}

include!("../../test_support/integration/process_file_lock_fault_helpers.rs");

fn hold_shared(file: File, identity: ObjectIdentity) {
    let _shared = ProcessFileLock::lock_shared(&file, identity).unwrap();
    let ready = PathBuf::from(std::env::var_os(HELPER_READY_ENV).unwrap());
    let release = PathBuf::from(std::env::var_os(HELPER_RELEASE_ENV).unwrap());
    fs::write(ready, PROCESS_FILE_LOCK_TEST_SIGNAL_BYTES).unwrap();
    let deadline = Instant::now() + HELPER_TIMEOUT;
    while !release.exists() {
        assert!(Instant::now() < deadline, "{HELPER_RELEASE_TIMEOUT}");
        std::thread::sleep(HELPER_POLL_INTERVAL);
    }
}

pub(super) fn helper_command(mode: &str, path: &Path) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg(PROCESS_FILE_LOCK_TEST_HELPER_FILTER)
        .arg(PROCESS_FILE_LOCK_TEST_NOCAPTURE_ARGUMENT)
        .env(HELPER_MODE_ENV, mode)
        .env(HELPER_PATH_ENV, path);
    command
}

fn wait_for_holder(ready: &Path, child: &mut Child) {
    let deadline = Instant::now() + HELPER_TIMEOUT;
    while !ready.exists() {
        assert!(child.try_wait().unwrap().is_none(), "{HOLDER_EXITED_EARLY}");
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{HOLDER_READINESS_TIMEOUT}");
        }
        std::thread::sleep(HELPER_POLL_INTERVAL);
    }
}

fn create_lock_file(path: &Path) {
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
}

fn open_lock_file(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap()
}
