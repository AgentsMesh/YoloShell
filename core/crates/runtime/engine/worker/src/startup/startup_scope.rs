use super::constants::{STARTUP_CHILD_EXIT_POLL_INTERVAL, STARTUP_SCOPE_MONITOR_THREAD_NAME};
use crate::process::{CapturedProcessTree, ProcessIdentity};
use anyhow::{Context, Result, bail};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub(crate) type TrackedTree = Arc<Mutex<CapturedProcessTree>>;
pub(crate) type MonitorFailure = Arc<Mutex<Option<String>>>;

pub(crate) const MONITOR_FAILURE_CONTEXT: &str = "startup process-scope monitor refresh failed";
pub(crate) const MONITOR_PANIC_DETAIL: &str = "startup process-scope monitor panicked";

pub(crate) fn new_tracked_tree(tree: CapturedProcessTree) -> TrackedTree {
    Arc::new(Mutex::new(tree))
}

pub(crate) fn refresh(tree: &TrackedTree) -> Result<Vec<ProcessIdentity>> {
    lock(tree).refresh()
}

#[cfg(test)]
pub(crate) fn snapshot(tree: &TrackedTree) -> Vec<ProcessIdentity> {
    lock(tree).identities()
}

#[cfg(test)]
pub(crate) fn contains(tree: &TrackedTree, identity: ProcessIdentity) -> bool {
    snapshot(tree).contains(&identity)
}

pub(crate) struct StartupScopeMonitor {
    pub(crate) stop: Arc<AtomicBool>,
    pub(crate) failure: MonitorFailure,
    pub(crate) handle: Option<std::thread::JoinHandle<()>>,
}

impl StartupScopeMonitor {
    pub(crate) fn start(tree: TrackedTree) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let failure = Arc::new(Mutex::new(None));
        let thread_stop = Arc::clone(&stop);
        let thread_failure = Arc::clone(&failure);
        let handle = std::thread::Builder::new()
            .name(STARTUP_SCOPE_MONITOR_THREAD_NAME.to_owned())
            .spawn(move || monitor(tree, thread_stop, thread_failure))
            .context("spawn startup process-scope monitor")?;
        Ok(Self {
            stop,
            failure,
            handle: Some(handle),
        })
    }

    pub(crate) fn check_failure(&self) -> Result<()> {
        if let Some(detail) = lock_failure(&self.failure).as_deref() {
            bail!("{MONITOR_FAILURE_CONTEXT}: {detail}");
        }
        Ok(())
    }

    pub(crate) fn stop(&mut self) -> Result<()> {
        self.request_stop();
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!(MONITOR_PANIC_DETAIL))?;
        }
        self.check_failure()
    }

    pub(crate) fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
    }

    #[cfg(test)]
    pub(crate) fn failed_for_test(detail: &str) -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(true)),
            failure: Arc::new(Mutex::new(Some(detail.to_owned()))),
            handle: None,
        }
    }
}

impl Drop for StartupScopeMonitor {
    fn drop(&mut self) {
        self.request_stop();
        if let Some(handle) = self.handle.take() {
            super::startup_cleanup_owner::detach(vec![handle]);
        }
    }
}

pub(crate) fn monitor(tree: TrackedTree, stop: Arc<AtomicBool>, failure: MonitorFailure) {
    while !stop.load(Ordering::Acquire) {
        if let Err(error) = refresh(&tree) {
            *lock_failure(&failure) = Some(error.to_string());
            return;
        }
        std::thread::sleep(STARTUP_CHILD_EXIT_POLL_INTERVAL);
    }
}

pub(crate) fn lock(tree: &TrackedTree) -> std::sync::MutexGuard<'_, CapturedProcessTree> {
    tree.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn lock_failure(failure: &MonitorFailure) -> std::sync::MutexGuard<'_, Option<String>> {
    failure
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
