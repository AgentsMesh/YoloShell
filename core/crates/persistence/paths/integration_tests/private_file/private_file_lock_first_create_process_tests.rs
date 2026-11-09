use super::*;
use crate::paths::constants::{
    PRIVATE_FILE_LOCK_FIRST_CREATE_HELPER_FILTER, PROCESS_FILE_LOCK_TEST_NOCAPTURE_ARGUMENT,
    PROCESS_FILE_LOCK_TEST_SIGNAL_BYTES,
};
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

const FLOCK_SUCCESS: libc::c_int = 0;
const FIRST_CREATION_RACE_FILE_NAME: &str = "first-creation-race.lock";
const HELPER_TIMEOUT: Duration = Duration::from_secs(5);
const INTERMEDIATE_FILE_MODE: u32 = 0o400;
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const RETRY_DELAY: Duration = Duration::from_millis(2);
const LOCK_MAX_BYTES: u64 = 0;
const HELPER_MODE: &str = "race-first-private-file-lock";
const MODE_ENV: &str = "YOLO_PRIVATE_FIRST_CREATE_MODE";
const PATH_ENV: &str = "YOLO_PRIVATE_FIRST_CREATE_PATH";
const MARKER_ENV: &str = "YOLO_PRIVATE_FIRST_CREATE_MARKER";
const PEER_ENV: &str = "YOLO_PRIVATE_FIRST_CREATE_PEER";
const ACQUIRED_ENV: &str = "YOLO_PRIVATE_FIRST_CREATE_ACQUIRED";
const RELEASE_ENV: &str = "YOLO_PRIVATE_FIRST_CREATE_RELEASE";
const LOCK_FILE: &str = "first-create.lock";

#[test]
fn concurrent_processes_retry_the_first_lock_creation_collision() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(LOCK_FILE);
    crate::paths::ensure_private_parent(&path).unwrap();
    let first_marker = temporary.path().join("first.marker");
    let second_marker = temporary.path().join("second.marker");
    let first_acquired = temporary.path().join("first.acquired");
    let second_acquired = temporary.path().join("second.acquired");
    let release = temporary.path().join("release");
    let mut first = launch(
        &path,
        &first_marker,
        &second_marker,
        &first_acquired,
        &release,
    );
    let mut second = launch(
        &path,
        &second_marker,
        &first_marker,
        &second_acquired,
        &release,
    );

    wait_for_any(&mut first, &mut second, &first_acquired, &second_acquired);
    std::fs::write(&release, PROCESS_FILE_LOCK_TEST_SIGNAL_BYTES).unwrap();
    assert!(first.wait().unwrap().success());
    assert!(second.wait().unwrap().success());
    assert!(first_acquired.exists());
    assert!(second_acquired.exists());
}

#[test]
fn first_creation_eexist_is_retried_until_one_lock_wins() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(FIRST_CREATION_RACE_FILE_NAME);
    crate::paths::ensure_private_parent(&path).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let spawn = |barrier: Arc<Barrier>| {
        let path = path.clone();
        std::thread::spawn(move || {
            crate::paths::private_file::private_file_missing::set_before_create_hook(move || {
                barrier.wait();
            });
            let directory = PrivateDirectory::open(path.parent().unwrap())?;
            PrivateFileLock::acquire_in_wait(
                directory,
                &path,
                LOCK_MAX_BYTES,
                HELPER_TIMEOUT,
                RETRY_DELAY,
            )
            .map(drop)
        })
    };

    let first = spawn(Arc::clone(&barrier));
    let second = spawn(barrier);
    first.join().unwrap().unwrap();
    second.join().unwrap().unwrap();
}

#[test]
fn waited_lock_rejects_ctime_drift_for_the_same_object() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(LOCK_FILE);
    crate::paths::ensure_private_parent(&path).unwrap();
    let entry = PrivateFileEntry::ensure(&path).unwrap();
    let opened = entry.open_rw_or_create(LOCK_MAX_BYTES).unwrap();
    let original = opened.identity;
    assert_eq!(
        unsafe { libc::flock(opened.file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        FLOCK_SUCCESS
    );
    let holder = opened.file;
    let (observed_tx, observed_rx) = std::sync::mpsc::channel();
    let waiter_path = path.clone();
    let waiter = std::thread::spawn(move || {
        PrivateFileLock::set_wait_contention_hook(move || observed_tx.send(()).unwrap());
        let directory = PrivateDirectory::open(waiter_path.parent().unwrap()).unwrap();
        PrivateFileLock::acquire_in_wait(
            directory,
            &waiter_path,
            LOCK_MAX_BYTES,
            HELPER_TIMEOUT,
            RETRY_DELAY,
        )
    });
    observed_rx.recv_timeout(HELPER_TIMEOUT).unwrap();
    std::fs::set_permissions(
        &path,
        std::fs::Permissions::from_mode(INTERMEDIATE_FILE_MODE),
    )
    .unwrap();
    std::fs::set_permissions(
        &path,
        std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    let settled = FileIdentity::inspect_file_bounded(&holder, &path, LOCK_MAX_BYTES).unwrap();
    assert!(settled.same_object(original));
    assert!(settled.same_content_version(original));
    assert_ne!(settled, original);
    drop(holder);
    assert!(waiter.join().unwrap().is_err());
}

#[test]
fn private_file_lock_first_create_subprocess_helper() {
    if std::env::var_os(MODE_ENV).as_deref() != Some(HELPER_MODE.as_ref()) {
        return;
    }
    let path = env_path(PATH_ENV);
    let marker = env_path(MARKER_ENV);
    let peer = env_path(PEER_ENV);
    let acquired = env_path(ACQUIRED_ENV);
    let release = env_path(RELEASE_ENV);
    crate::paths::private_file::private_file_missing::set_before_create_hook(move || {
        std::fs::write(marker, PROCESS_FILE_LOCK_TEST_SIGNAL_BYTES).unwrap();
        wait_for_path(&peer);
    });
    let directory = PrivateDirectory::open(path.parent().unwrap()).unwrap();
    let _lock = PrivateFileLock::acquire_in_wait(
        directory,
        &path,
        LOCK_MAX_BYTES,
        HELPER_TIMEOUT,
        RETRY_DELAY,
    )
    .unwrap();
    std::fs::write(acquired, PROCESS_FILE_LOCK_TEST_SIGNAL_BYTES).unwrap();
    wait_for_path(&release);
}

fn launch(path: &Path, marker: &Path, peer: &Path, acquired: &Path, release: &Path) -> Child {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg(PRIVATE_FILE_LOCK_FIRST_CREATE_HELPER_FILTER)
        .arg(PROCESS_FILE_LOCK_TEST_NOCAPTURE_ARGUMENT)
        .env(MODE_ENV, HELPER_MODE)
        .env(PATH_ENV, path)
        .env(MARKER_ENV, marker)
        .env(PEER_ENV, peer)
        .env(ACQUIRED_ENV, acquired)
        .env(RELEASE_ENV, release);
    crate::process::spawn_waitable_test_child(command)
}

fn wait_for_any(first: &mut Child, second: &mut Child, left: &Path, right: &Path) {
    let deadline = Instant::now() + HELPER_TIMEOUT;
    while !left.exists() && !right.exists() {
        assert!(first.try_wait().unwrap().is_none());
        assert!(second.try_wait().unwrap().is_none());
        assert!(Instant::now() < deadline);
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_path(path: &Path) {
    let deadline = Instant::now() + HELPER_TIMEOUT;
    while !path.exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn env_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}
