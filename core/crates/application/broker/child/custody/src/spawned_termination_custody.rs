use super::{TerminationReservation, lock_queue};
use anyhow::Result;
use std::time::Instant;

const REAPER_WORKER_FAILED: &str = "broker termination reaper worker failed";
const REAPER_WAIT_TIMEOUT: &str = "broker termination reaper wait timed out";

impl TerminationReservation {
    pub(crate) fn retain(
        mut self,
        child: yolo_process::DetachedChildCustody,
        termination_required: bool,
    ) {
        let mut queue = lock_queue(&self.shared);
        if termination_required {
            queue.termination_count += 1;
        }
        queue.children.push_back(super::RetainedChild {
            child,
            termination_required,
        });
        self.active = false;
        drop(queue);
        self.shared.ready.notify_one();
    }
}

impl Drop for TerminationReservation {
    fn drop(&mut self) {
        if self.active {
            lock_queue(&self.shared).custody_count -= 1;
        }
    }
}

pub fn wait_for_termination_delivery(deadline: Instant) -> Result<()> {
    wait_for(deadline, |queue| queue.termination_count)
}

pub fn wait_for_custody_settlement(deadline: Instant) -> Result<()> {
    wait_for(deadline, |queue| queue.custody_count)
}

fn wait_for(deadline: Instant, pending: impl Fn(&super::Queue) -> usize) -> Result<()> {
    let Some(Ok(reaper)) = super::REAPER.get() else {
        return Ok(());
    };
    let mut queue = lock_queue(&reaper.shared);
    while pending(&queue) > usize::default() {
        anyhow::ensure!(!queue.worker_failed, REAPER_WORKER_FAILED);
        let now = Instant::now();
        anyhow::ensure!(now < deadline, REAPER_WAIT_TIMEOUT);
        let remaining = deadline
            .saturating_duration_since(now)
            .min(yolo_remote_broker::BROKER_READY_POLL_INTERVAL);
        let (next, _) = reaper
            .shared
            .ready
            .wait_timeout(queue, remaining)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        queue = next;
    }
    anyhow::ensure!(!queue.worker_failed, REAPER_WORKER_FAILED);
    Ok(())
}

#[cfg(test)]
impl super::TerminationReaper {
    pub(super) fn wait_for_termination_delivery(&self, deadline: Instant) -> Result<()> {
        let mut queue = lock_queue(&self.shared);
        while queue.termination_count > usize::default() {
            anyhow::ensure!(!queue.worker_failed, REAPER_WORKER_FAILED);
            let remaining = deadline.saturating_duration_since(Instant::now());
            anyhow::ensure!(!remaining.is_zero(), REAPER_WAIT_TIMEOUT);
            let (next, _) = self
                .shared
                .ready
                .wait_timeout(queue, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            queue = next;
        }
        Ok(())
    }
}
