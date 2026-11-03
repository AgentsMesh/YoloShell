use super::{
    DetachedChildReservation, REAPER, REAPER_POLL_INTERVAL, RetainedChild, TerminationPolicy,
    lock_queue,
};
use anyhow::Result;
use std::time::Instant;

const REAPER_WORKER_FAILED: &str = "detached child reaper worker failed";
const TERMINATION_DELIVERY_TIMEOUT: &str = "detached child termination delivery timed out";

impl DetachedChildReservation {
    pub(in crate::process) fn retain(
        mut self,
        child: crate::process::DetachedChildCustody,
        policy: TerminationPolicy,
    ) {
        let mut queue = lock_queue(&self.shared);
        if policy == TerminationPolicy::Terminate {
            queue.termination_count += 1;
        }
        queue.children.push_back(RetainedChild { child, policy });
        self.active = false;
        drop(queue);
        self.shared.ready.notify_one();
    }
}

impl Drop for DetachedChildReservation {
    fn drop(&mut self) {
        if self.active {
            lock_queue(&self.shared).custody_count -= 1;
        }
    }
}

pub fn wait_for_termination_delivery(deadline: Instant) -> Result<()> {
    let Some(Ok(reaper)) = REAPER.get() else {
        return Ok(());
    };
    wait_for(reaper, deadline)
}

fn wait_for(reaper: &super::DetachedChildReaper, deadline: Instant) -> Result<()> {
    let shared = &reaper.shared;
    let mut queue = lock_queue(shared);
    while queue.termination_count > usize::default() {
        anyhow::ensure!(!queue.worker_failed, REAPER_WORKER_FAILED);
        let now = Instant::now();
        anyhow::ensure!(now < deadline, TERMINATION_DELIVERY_TIMEOUT);
        let remaining = deadline
            .saturating_duration_since(now)
            .min(REAPER_POLL_INTERVAL);
        let (next, _) = shared
            .ready
            .wait_timeout(queue, remaining)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        queue = next;
    }
    anyhow::ensure!(!queue.worker_failed, REAPER_WORKER_FAILED);
    Ok(())
}

#[cfg(test)]
impl super::DetachedChildReaper {
    pub(super) fn wait_for_termination_delivery(&self, deadline: Instant) -> Result<()> {
        wait_for(self, deadline)
    }
}
