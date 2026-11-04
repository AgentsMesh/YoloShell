use anyhow::{Context, Result, ensure};
use std::time::Duration;

const START_DEADLINE_OVERFLOW: &str = "broker start deadline overflow";
const START_DEADLINE_EXPIRED: &str = "broker start deadline expired";

pub(super) struct ReconcileDeadline {
    active: tokio::time::Instant,
}

pub(super) struct LaunchAdmission {
    boundary: tokio::time::Instant,
}

impl ReconcileDeadline {
    pub(super) fn until(active: tokio::time::Instant) -> Result<Self> {
        let result = Self { active };
        result.verify()?;
        Ok(result)
    }
    pub(super) fn capture(timeout: Duration) -> Result<Self> {
        Ok(Self {
            active: tokio::time::Instant::now()
                .checked_add(timeout)
                .context(START_DEADLINE_OVERFLOW)?,
        })
    }

    pub(super) fn active(&self) -> tokio::time::Instant {
        self.active
    }

    pub(super) fn exit_delivery(&self) -> std::time::Instant {
        self.active.into_std()
    }

    pub(super) fn verify(&self) -> Result<()> {
        ensure!(
            tokio::time::Instant::now() < self.active,
            START_DEADLINE_EXPIRED
        );
        Ok(())
    }

    pub(super) fn prepare_launch(&self) -> Result<LaunchAdmission> {
        self.verify()?;
        Ok(LaunchAdmission {
            boundary: self.active,
        })
    }

    pub(super) async fn wait_for_next_attempt(&self, interval: Duration) {
        let wake = tokio::time::Instant::now()
            .checked_add(interval)
            .map(|candidate| candidate.min(self.active))
            .unwrap_or(self.active);
        tokio::time::sleep_until(wake).await;
    }
}

impl LaunchAdmission {
    pub(super) fn deadline(&self) -> tokio::time::Instant {
        self.boundary
    }

    pub(super) fn verify(&self) -> Result<()> {
        ensure!(
            tokio::time::Instant::now() < self.boundary,
            START_DEADLINE_EXPIRED
        );
        Ok(())
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/start_deadline_tests.rs"]
mod tests;
