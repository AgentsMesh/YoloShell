use crate::{Federation, ObservationHistory, constants::CLOSED};
use anyhow::{Result, ensure};
use std::sync::atomic::Ordering;

impl Federation {
    /// Stops admission and publication; the supervisor still owns joining network tasks.
    pub fn begin_shutdown(&self) {
        self.lifetime.retire();
        self.closing.store(true, Ordering::Release);
        self.observation_cache.retire();
        self.backend.close_files();
    }

    /// Network-task completion precedes this drain of protocol-owned resources.
    pub async fn wait_retired(&self, deadline: tokio::time::Instant) -> Result<()> {
        self.lifetime.drained(deadline).await
    }

    pub(crate) fn is_closing(&self) -> bool {
        self.closing.load(Ordering::Acquire)
    }

    pub(crate) fn require_open(&self) -> Result<()> {
        ensure!(!self.is_closing(), CLOSED);
        self.state.verify()
    }

    pub async fn history(&self) -> ObservationHistory {
        self.book.lock().await.history()
    }

    pub async fn restore_history(&self, history: ObservationHistory) -> Result<()> {
        self.require_open()?;
        let mut book = self.book.lock().await;
        self.require_open()?;
        book.restore_history(history)?;
        let _ = self.observation_cache.record(book.history());
        self.require_open()
    }

    pub async fn copy_history_from(&self, previous: &Self) -> Result<()> {
        let history = previous.history().await;
        self.restore_history(history).await
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/retirement.rs"]
mod unit_tests;
