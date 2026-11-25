use super::PendingBroker;
use anyhow::{Context, Result};
use std::process::ExitStatus;
use std::time::{Duration, Instant};

const WAIT_CHILD_CONTEXT: &str = "wait for detached broker child completion";
pub(super) const WAIT_CHILD_TIMEOUT_DETAIL: &str = "detached broker child completion timed out";
const WAIT_CHILD_DEADLINE_DETAIL: &str = "detached broker child deadline overflow";
const WAIT_CHILD_SETTLEMENT_DEADLINE_DETAIL: &str =
    "detached broker child settlement deadline overflow";
const WAIT_CHILD_TERMINATION_PREFIX: &str = "; child termination failed: ";

impl PendingBroker {
    pub(crate) fn wait_for_exit(mut self, timeout: Duration) -> Result<ExitStatus> {
        self.compensation.take();
        let deadline = Instant::now()
            .checked_add(timeout)
            .context(WAIT_CHILD_DEADLINE_DETAIL)?;
        loop {
            let child = &mut self.detached.as_mut().context(WAIT_CHILD_CONTEXT)?.child;
            if let Some(status) = child.try_wait().context(WAIT_CHILD_CONTEXT)? {
                self.detached.take();
                return Ok(status);
            }
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let remaining = deadline.saturating_duration_since(now);
            std::thread::sleep(remaining.min(yolo_remote_broker::BROKER_READY_POLL_INTERVAL));
        }
        self.timeout_child()
    }

    fn timeout_child(&mut self) -> Result<ExitStatus> {
        let settlement_deadline = Instant::now()
            .checked_add(yolo_remote_broker::BROKER_READY_TIMEOUT)
            .context(WAIT_CHILD_SETTLEMENT_DEADLINE_DETAIL)?;
        let detached = self.detached.take().context(WAIT_CHILD_CONTEXT)?;
        let settlement = super::settlement::settle_child(detached.child, detached.termination);
        let reaped = super::termination::wait_for_custody_settlement(settlement_deadline);
        let cleanup = super::result::combine_cleanup_results(settlement, reaped);
        match cleanup {
            Ok(()) => anyhow::bail!(WAIT_CHILD_TIMEOUT_DETAIL),
            Err(cleanup) => anyhow::bail!(
                "{WAIT_CHILD_TIMEOUT_DETAIL}{WAIT_CHILD_TERMINATION_PREFIX}{cleanup:#}"
            ),
        }
    }
}
