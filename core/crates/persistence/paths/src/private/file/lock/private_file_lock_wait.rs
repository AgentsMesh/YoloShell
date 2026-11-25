use super::*;
use anyhow::{Context, Result, ensure};
use std::time::{Duration, Instant};

const INVALID_RETRY_DELAY: &str = "private file lock retry delay must be positive";
const TIMEOUT_CONTEXT: &str = "private file lock acquisition timed out";
const WAIT_DEADLINE_OVERFLOW: &str = "private file lock wait deadline overflow";
const WAIT_VERSION_CHANGED: &str = "waited private file lock version changed";
const WAIT_DESCRIPTOR_CONTEXT: &str = "inspect waited private file lock descriptor";
const WAIT_VERIFY_CONTEXT: &str = "verify waited private file lock path";

impl PrivateFileLock {
    pub fn identity(&self) -> FileIdentity {
        self.identity
    }

    pub fn acquire_in_wait(
        directory: PrivateDirectory,
        path: &Path,
        max_bytes: u64,
        timeout: Duration,
        retry_delay: Duration,
    ) -> Result<Self> {
        ensure!(!retry_delay.is_zero(), INVALID_RETRY_DELAY);
        let deadline = Instant::now()
            .checked_add(timeout)
            .context(WAIT_DEADLINE_OVERFLOW)?;
        let entry = PrivateFileEntry::from_directory(directory, path)?;
        let opened = loop {
            match entry.open_rw_or_create(max_bytes) {
                Ok(opened) => break opened,
                Err(error) if crate::paths::private_file::is_creation_contention(&error) => {
                    wait_for_retry(deadline, retry_delay)?;
                }
                Err(error) => return Err(error),
            }
        };
        acquire_opened(entry, opened, max_bytes, deadline, retry_delay)
    }

    pub fn acquire_existing_in_wait(
        directory: PrivateDirectory,
        path: &Path,
        max_bytes: u64,
        timeout: Duration,
        retry_delay: Duration,
    ) -> Result<Option<Self>> {
        ensure!(!retry_delay.is_zero(), INVALID_RETRY_DELAY);
        let deadline = Instant::now()
            .checked_add(timeout)
            .context(WAIT_DEADLINE_OVERFLOW)?;
        let entry = PrivateFileEntry::from_directory(directory, path)?;
        let opened = match entry.open_rw(max_bytes) {
            Ok(opened) => opened,
            Err(error)
                if crate::paths::private_file::io_kind(&error)
                    == Some(std::io::ErrorKind::NotFound) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        acquire_opened(entry, opened, max_bytes, deadline, retry_delay).map(Some)
    }
}

fn acquire_opened(
    entry: PrivateFileEntry,
    opened: OpenedPrivateFile,
    max_bytes: u64,
    deadline: Instant,
    retry_delay: Duration,
) -> Result<PrivateFileLock> {
    let identity = opened.identity.object_identity();
    let turn = ProcessLockTurn::acquire_until(identity, deadline)?.context(TIMEOUT_CONTEXT)?;
    let mut contention_observed = false;
    loop {
        if let Some(process_lock) =
            ProcessFileLock::try_lock(&opened.file, identity, Mode::Exclusive)?
        {
            ensure!(Instant::now() < deadline, TIMEOUT_CONTEXT);
            let settled = FileIdentity::inspect_file_bounded(&opened.file, entry.path(), max_bytes)
                .context(WAIT_DESCRIPTOR_CONTEXT)?;
            ensure!(settled.unchanged(opened.identity), WAIT_VERSION_CHANGED);
            entry
                .verify_current_unchanged(settled, max_bytes)
                .context(WAIT_VERIFY_CONTEXT)?;
            return Ok(PrivateFileLock {
                entry,
                _process_lock: process_lock,
                _turn: turn,
                file: opened.file,
                identity: settled,
                max_bytes,
            });
        }
        if !contention_observed {
            run_contention_hook();
            contention_observed = true;
        }
        wait_for_retry(deadline, retry_delay)?;
        run_retry_hook(deadline);
    }
}

fn wait_for_retry(deadline: Instant, retry_delay: Duration) -> Result<()> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .context(TIMEOUT_CONTEXT)?;
    std::thread::sleep(remaining.min(retry_delay));
    ensure!(Instant::now() < deadline, TIMEOUT_CONTEXT);
    Ok(())
}

#[cfg(any(test, feature = "persistence-test-support"))]
thread_local! {
    static CONTENTION_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const {
        std::cell::RefCell::new(None)
    };
    static RETRY_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce(Instant)>>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(any(test, feature = "persistence-test-support"))]
impl PrivateFileLock {
    pub(crate) fn set_wait_contention_hook(hook: impl FnOnce() + 'static) {
        CONTENTION_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
    }

    pub(crate) fn set_wait_retry_hook(hook: impl FnOnce(Instant) + 'static) {
        RETRY_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
    }
}

#[cfg(any(test, feature = "persistence-test-support"))]
fn run_contention_hook() {
    CONTENTION_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(any(test, feature = "persistence-test-support"))]
fn run_retry_hook(deadline: Instant) {
    RETRY_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook(deadline);
        }
    });
}

#[cfg(not(any(test, feature = "persistence-test-support")))]
fn run_contention_hook() {}

#[cfg(not(any(test, feature = "persistence-test-support")))]
fn run_retry_hook(_deadline: Instant) {}
