use crate::paths::process_file_lock::{Mode, ProcessFileLock};
use anyhow::{Context, Result, ensure};
use std::time::{Duration, Instant};

const INVALID_RETRY: &str = "retained private file lock retry delay must be positive";
const DEADLINE_OVERFLOW: &str = "retained private file lock deadline overflow";
const LOCK_TIMEOUT: &str = "retained private file lock acquisition timed out";
const DESCRIPTOR_CHANGED: &str = "retained private file descriptor changed while waiting";
const PATH_CHANGED: &str = "retained private file path changed while waiting";

impl PrivateFileEntry {
    pub fn open_retained_with_shared_lock_until(
        &self,
        max_bytes: u64,
        timeout: Duration,
        retry_delay: Duration,
    ) -> Result<(OpenedPrivateFile, ProcessFileLock)> {
        let opened = self.open_retained(max_bytes)?;
        let lock =
            self.lock_opened_retained_shared_until(&opened, max_bytes, timeout, retry_delay)?;
        Ok((opened, lock))
    }

    pub(crate) fn lock_opened_retained_shared_until(
        &self,
        opened: &OpenedPrivateFile,
        max_bytes: u64,
        timeout: Duration,
        retry_delay: Duration,
    ) -> Result<ProcessFileLock> {
        ensure!(!retry_delay.is_zero(), INVALID_RETRY);
        let deadline = Instant::now()
            .checked_add(timeout)
            .context(DEADLINE_OVERFLOW)?;
        let object = opened.identity.object_identity();
        if timeout.is_zero() {
            let lock = ProcessFileLock::try_lock(&opened.file, object, Mode::Shared)?
                .context(LOCK_TIMEOUT)?;
            return self.verify_retained_shared_lock(opened, max_bytes, lock);
        }
        loop {
            if let Some(lock) = ProcessFileLock::try_lock(&opened.file, object, Mode::Shared)? {
                ensure!(Instant::now() < deadline, LOCK_TIMEOUT);
                return self.verify_retained_shared_lock(opened, max_bytes, lock);
            }
            wait_for_retry(deadline, retry_delay)?;
            run_retry_hook(deadline);
        }
    }

    fn verify_retained_shared_lock(
        &self,
        opened: &OpenedPrivateFile,
        max_bytes: u64,
        lock: ProcessFileLock,
    ) -> Result<ProcessFileLock> {
        let settled = FileIdentity::inspect_file_bounded(&opened.file, self.path(), max_bytes)?;
        ensure!(settled.unchanged(opened.identity), DESCRIPTOR_CHANGED);
        self.verify_retained_path_unchanged(settled, max_bytes)
            .context(PATH_CHANGED)?;
        Ok(lock)
    }
}

fn wait_for_retry(deadline: Instant, retry_delay: Duration) -> Result<()> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .context(LOCK_TIMEOUT)?;
    std::thread::sleep(remaining.min(retry_delay));
    ensure!(Instant::now() < deadline, LOCK_TIMEOUT);
    Ok(())
}

#[cfg(test)]
thread_local! {
    static RETRY_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce(Instant)>>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(test)]
impl PrivateFileEntry {
    pub(crate) fn set_retained_lock_retry_hook(hook: impl FnOnce(Instant) + 'static) {
        RETRY_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
    }
}

#[cfg(test)]
fn run_retry_hook(deadline: Instant) {
    RETRY_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook(deadline);
        }
    });
}

#[cfg(not(test))]
fn run_retry_hook(_deadline: Instant) {}
