use super::NamespaceLossContainment;
use crate::process::{MacosSecondaryCustody, ProcessIdentity};
use anyhow::{Context, Result, ensure};
use std::time::Duration;
use tokio::time::Instant;

const AUTHORITY_MISSING_DETAIL: &str = "namespace-loss holder discovery authority is unavailable";
const INITIAL_DISCOVERY_BACKOFF: Duration =
    crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;
const MAXIMUM_DISCOVERY_BACKOFF: Duration = crate::session_timing::PTY_MISSING_EOF_TIMEOUT;

pub(super) struct LateHolderDiscovery {
    next_scan: Option<Instant>,
    delay: Duration,
}

impl NamespaceLossContainment {
    pub(super) fn initialize_custody(&mut self, custody: &MacosSecondaryCustody) -> Result<()> {
        if self.authority.is_some() {
            return Ok(());
        }
        let (authority, holders) = self.capture_holders(custody)?;
        ensure!(
            authority.authorizes(&self.runtime),
            super::TARGET_RECONCILIATION_CONTEXT
        );
        self.track(holders)?;
        self.authority = Some(authority);
        Ok(())
    }

    pub(super) fn refresh_finalized_holders(
        &mut self,
        custody: &MacosSecondaryCustody,
    ) -> Result<()> {
        let discovery_clock = Instant::now();
        if !self.late_discovery.should_scan(discovery_clock) {
            return Ok(());
        }
        let (observed, holders) = self.capture_holders(custody)?;
        ensure!(
            self.authority()?.same_authority(&observed),
            super::TARGET_RECONCILIATION_CONTEXT
        );
        let discovered = !holders.is_empty();
        self.track(holders)?;
        self.late_discovery
            .record_success(discovery_clock, discovered);
        Ok(())
    }

    pub(super) fn authority(
        &self,
    ) -> Result<&crate::process::MacosNamespaceLossContainmentAuthority> {
        self.authority.as_ref().context(AUTHORITY_MISSING_DETAIL)
    }

    fn capture_holders(
        &mut self,
        custody: &MacosSecondaryCustody,
    ) -> Result<(
        crate::process::MacosNamespaceLossContainmentAuthority,
        Vec<ProcessIdentity>,
    )> {
        #[cfg(test)]
        {
            self.holder_discovery_scans = self.holder_discovery_scans.saturating_add(1);
        }
        custody.namespace_loss_containment_snapshot()
    }
}

impl LateHolderDiscovery {
    pub(super) fn arm(&mut self, now: Instant) {
        self.delay = INITIAL_DISCOVERY_BACKOFF;
        self.next_scan = Some(now + self.delay);
    }

    fn should_scan(&self, now: Instant) -> bool {
        self.next_scan.is_some_and(|deadline| now >= deadline)
    }

    fn record_success(&mut self, now: Instant, discovered: bool) {
        if discovered {
            self.delay = INITIAL_DISCOVERY_BACKOFF;
        }
        self.next_scan = Some(now + self.delay);
        if !discovered {
            self.delay = self
                .delay
                .saturating_add(self.delay)
                .min(MAXIMUM_DISCOVERY_BACKOFF);
        }
    }
}

impl Default for LateHolderDiscovery {
    fn default() -> Self {
        Self {
            next_scan: None,
            delay: INITIAL_DISCOVERY_BACKOFF,
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/settlement/settlement_containment_discovery_tests.rs"]
mod tests;
