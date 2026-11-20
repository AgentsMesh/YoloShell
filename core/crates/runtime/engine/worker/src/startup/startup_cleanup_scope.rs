use super::startup_cleanup_owner::StartupCleanupOwner;
use anyhow::{Context, Result, anyhow};

pub(crate) const REAPER_THREAD_PANIC_DETAIL: &str = "startup cleanup reaper thread panicked";
pub(crate) const SETTLEMENT_CHANNEL_CLOSED_DETAIL: &str =
    "startup cleanup owner stopped before reporting settlement";

pub(crate) struct StartupCleanupScope {
    pub(crate) owner: StartupCleanupOwner,
    pub(crate) reapers: Vec<std::thread::JoinHandle<()>>,
}

impl StartupCleanupScope {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            owner: StartupCleanupOwner::capture()?,
            reapers: Vec::new(),
        })
    }

    pub(crate) fn register(&mut self, reaper: std::thread::JoinHandle<()>) {
        self.reapers.push(reaper);
    }

    pub(crate) async fn settle(mut self) -> Result<()> {
        let receiver = self.owner.submit(std::mem::take(&mut self.reapers))?;
        receiver
            .await
            .map_err(|_| anyhow!(SETTLEMENT_CHANNEL_CLOSED_DETAIL))?
            .map_err(anyhow::Error::msg)
            .context(REAPER_THREAD_PANIC_DETAIL)
    }
}

impl Drop for StartupCleanupScope {
    fn drop(&mut self) {
        if !self.reapers.is_empty() {
            self.owner.detach(std::mem::take(&mut self.reapers));
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../unit_tests/startup/startup_cleanup_scope_tests.rs"]
mod tests;
