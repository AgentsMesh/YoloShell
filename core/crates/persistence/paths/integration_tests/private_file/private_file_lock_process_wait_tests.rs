use super::*;
use crate::paths::constants::{
    PRIVATE_FILE_LOCK_WAIT_TEST_HELPER_FILTER, PROCESS_FILE_LOCK_TEST_SIGNAL_BYTES,
};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const BOUNDED_TIMEOUT: Duration = Duration::from_millis(60);
const HELPER_POLL_INTERVAL: Duration = Duration::from_millis(5);
const HELPER_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_DELAY: Duration = Duration::from_millis(2);
const LOCK_MAX_BYTES: u64 = 0;
const HELPER_MODE: &str = "hold-private-file-lock";
const HELPER_MODE_ENV: &str = "YOLO_PRIVATE_LOCK_WAIT_HELPER_MODE";
const HELPER_PATH_ENV: &str = "YOLO_PRIVATE_LOCK_WAIT_HELPER_PATH";
const HELPER_READY_ENV: &str = "YOLO_PRIVATE_LOCK_WAIT_HELPER_READY";
const HELPER_RELEASE_ENV: &str = "YOLO_PRIVATE_LOCK_WAIT_HELPER_RELEASE";
const LOCK_FILE_NAME: &str = "cross-process-private.lock";
const READY_FILE_NAME: &str = "holder.ready";
const RELEASE_FILE_NAME: &str = "holder.release";

#[test]
fn cross_process_flock_contention_is_retried_until_release() {
    let fixture = ProcessFixture::new();
    let holder = ExternalHolder::launch(&fixture);
    let (observed_tx, observed_rx) = std::sync::mpsc::channel();
    let path = fixture.path.clone();
    let waiter = std::thread::spawn(move || {
        PrivateFileLock::set_wait_contention_hook(move || observed_tx.send(()).unwrap());
        wait_lock(&path, HELPER_TIMEOUT).map(drop)
    });

    observed_rx.recv_timeout(HELPER_TIMEOUT).unwrap();
    holder.release();

    waiter.join().unwrap().unwrap();
}

#[test]
fn cross_process_timeout_fails_closed_and_preserves_the_holder() {
    let fixture = ProcessFixture::new();
    let mut holder = ExternalHolder::launch(&fixture);
    let started = Instant::now();

    let result = wait_lock(&fixture.path, BOUNDED_TIMEOUT);
    let elapsed = started.elapsed();

    assert!(result.is_err());
    assert!(elapsed >= BOUNDED_TIMEOUT);
    assert!(elapsed < HELPER_TIMEOUT);
    assert!(holder.child_running());
    assert!(
        PrivateFileLock::try_acquire(&fixture.path, LOCK_MAX_BYTES)
            .unwrap()
            .is_none()
    );
    holder.release();
    assert!(
        PrivateFileLock::try_acquire(&fixture.path, LOCK_MAX_BYTES)
            .unwrap()
            .is_some()
    );
}

#[test]
fn private_file_lock_wait_subprocess_helper() {
    let Some(mode) = std::env::var_os(HELPER_MODE_ENV) else {
        return;
    };
    assert_eq!(mode, HELPER_MODE);
    let path = std::path::PathBuf::from(std::env::var_os(HELPER_PATH_ENV).unwrap());
    let ready = std::path::PathBuf::from(std::env::var_os(HELPER_READY_ENV).unwrap());
    let release = std::path::PathBuf::from(std::env::var_os(HELPER_RELEASE_ENV).unwrap());
    let _holder = PrivateFileLock::try_acquire(&path, LOCK_MAX_BYTES)
        .unwrap()
        .unwrap();
    std::fs::write(ready, PROCESS_FILE_LOCK_TEST_SIGNAL_BYTES).unwrap();
    let deadline = Instant::now() + HELPER_TIMEOUT;
    while !release.exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(HELPER_POLL_INTERVAL);
    }
}

fn wait_lock(path: &Path, timeout: Duration) -> Result<PrivateFileLock> {
    let directory = PrivateDirectory::open(path.parent().unwrap())?;
    PrivateFileLock::acquire_in_wait(directory, path, LOCK_MAX_BYTES, timeout, RETRY_DELAY)
}

struct ExternalHolder {
    child: Option<Child>,
    release: std::path::PathBuf,
}

impl ExternalHolder {
    fn launch(fixture: &ProcessFixture) -> Self {
        let ready = fixture._temporary.path().join(READY_FILE_NAME);
        let release = fixture._temporary.path().join(RELEASE_FILE_NAME);
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .arg(PRIVATE_FILE_LOCK_WAIT_TEST_HELPER_FILTER)
            .arg(crate::paths::constants::PROCESS_FILE_LOCK_TEST_NOCAPTURE_ARGUMENT)
            .env(HELPER_MODE_ENV, HELPER_MODE)
            .env(HELPER_PATH_ENV, &fixture.path)
            .env(HELPER_READY_ENV, &ready)
            .env(HELPER_RELEASE_ENV, &release);
        let child = crate::process::spawn_waitable_test_child(command);
        let mut holder = Self {
            child: Some(child),
            release,
        };
        holder.wait_ready(&ready);
        holder
    }

    fn wait_ready(&mut self, ready: &Path) {
        let deadline = Instant::now() + HELPER_TIMEOUT;
        while !ready.exists() {
            assert!(self.child_running());
            assert!(Instant::now() < deadline);
            std::thread::sleep(HELPER_POLL_INTERVAL);
        }
    }

    fn child_running(&mut self) -> bool {
        self.child.as_mut().unwrap().try_wait().unwrap().is_none()
    }

    fn release(mut self) {
        std::fs::write(&self.release, PROCESS_FILE_LOCK_TEST_SIGNAL_BYTES).unwrap();
        let mut child = self.child.take().unwrap();
        assert!(child.wait().unwrap().success());
    }
}

impl Drop for ExternalHolder {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = std::fs::write(&self.release, PROCESS_FILE_LOCK_TEST_SIGNAL_BYTES);
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

struct ProcessFixture {
    _temporary: tempfile::TempDir,
    path: std::path::PathBuf,
}

impl ProcessFixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join(LOCK_FILE_NAME);
        let entry = PrivateFileEntry::ensure(&path).unwrap();
        drop(entry.open_rw_or_create(LOCK_MAX_BYTES).unwrap());
        Self {
            _temporary: temporary,
            path,
        }
    }
}
