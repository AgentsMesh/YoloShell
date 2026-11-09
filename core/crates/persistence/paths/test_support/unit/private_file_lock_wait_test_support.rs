use super::*;
use std::os::fd::AsRawFd;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub(super) fn wait_lock(path: &Path, timeout: Duration) -> Result<PrivateFileLock> {
    let parent = PrivateDirectory::open(path.parent().unwrap())?;
    PrivateFileLock::acquire_in_wait(parent, path, LOCK_MAX_BYTES, timeout, RETRY_DELAY)
}

pub(super) fn create_lock_file(path: &Path) {
    let entry = PrivateFileEntry::ensure(path).unwrap();
    drop(entry.open_rw_or_create(LOCK_MAX_BYTES).unwrap());
}

pub(super) fn spawn_waiter(
    path: std::path::PathBuf,
    order: usize,
    acquired: mpsc::Sender<usize>,
    release: mpsc::Receiver<()>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let lock = wait_lock(&path, TEST_TIMEOUT).unwrap();
        acquired.send(order).unwrap();
        release.recv_timeout(TEST_TIMEOUT).unwrap();
        drop(lock);
    })
}

pub(super) fn wait_for_queued(identity: crate::paths::identity::ObjectIdentity, expected: usize) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while ProcessLockTurn::queued_waiters_for_test(identity) != expected {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

pub(super) struct Fixture {
    _temporary_root: tempfile::TempDir,
    pub(super) path: std::path::PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join(LOCK_FILE_NAME);
        create_lock_file(&path);
        Self {
            _temporary_root: temporary,
            path,
        }
    }

    pub(super) fn kernel_holder(&self) -> std::fs::File {
        let entry = PrivateFileEntry::open(&self.path).unwrap();
        let opened = entry.open_rw(LOCK_MAX_BYTES).unwrap();
        assert_eq!(
            unsafe { libc::flock(opened.file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            FLOCK_SUCCESS
        );
        opened.file
    }
}
