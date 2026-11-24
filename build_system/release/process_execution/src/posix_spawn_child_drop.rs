use super::SpawnedProcess;
use crate::constants::{PROCESS_POLL_INTERVAL, PROCESS_SETTLEMENT_EPOCH_GRACE};
use crate::posix_supervisor::Streams;
use crate::process_group::{signal as signal_group, signal_process};
use crate::process_group_members::has_other_members;
use anyhow::{Result, anyhow, ensure};
use std::time::{Duration, Instant};
use yolo_process::process_discovery_force_grace;

const ROLLBACK_SURVIVAL_DETAIL: &str = "suspended process survived rollback deadline";
const ROLLBACK_REAP_CONTEXT: &str = "rollback reap failed";

impl SpawnedProcess {
    fn reap_before(&mut self, timeout: Duration) -> Result<bool> {
        let deadline = Instant::now() + timeout;
        while self.status.is_none() && Instant::now() < deadline {
            match self.exited() {
                Ok(true) => {
                    self.wait()?;
                    return Ok(true);
                }
                Err(error) => return Err(error),
                Ok(false) => std::thread::sleep(
                    PROCESS_POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())),
                ),
            }
        }
        Ok(self.status.is_some())
    }

    pub(crate) fn rollback_failed_capture(&mut self) -> Result<()> {
        let signal = self.signal_custody(libc::SIGKILL);
        let reap = self
            .reap_before(crate::constants::PROCESS_DROP_REAP_GRACE)
            .and_then(|reaped| {
                ensure!(reaped, ROLLBACK_SURVIVAL_DETAIL);
                Ok(())
            });
        if reap.is_ok() {
            self.group_settled = true;
            self.settle();
        }
        match (signal, reap) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
            (Err(signal), Err(reap)) => {
                Err(anyhow!("{signal:#}; {ROLLBACK_REAP_CONTEXT}: {reap:#}"))
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn configure_drop_fault_for_test(&mut self, reap_grace: Duration) {
        self.drop_signals_enabled = false;
        self.drop_reap_grace = reap_grace;
    }

    fn drop_signals_enabled(&self) -> bool {
        #[cfg(test)]
        return self.drop_signals_enabled;
        #[cfg(not(test))]
        true
    }

    fn drop_reap_grace(&self) -> Duration {
        #[cfg(test)]
        return self.drop_reap_grace;
        #[cfg(not(test))]
        crate::constants::PROCESS_DROP_REAP_GRACE
    }
}

impl Drop for SpawnedProcess {
    fn drop(&mut self) {
        if self.settled {
            return;
        }
        if self.drop_signals_enabled() && !self.group_settled {
            let _ = signal_group(self.pid, libc::SIGKILL);
        }
        if self.drop_signals_enabled() && self.status.is_none() {
            let _ = signal_process(self.pid, libc::SIGKILL);
        }
        let discovery_settled = self.custody.as_mut().is_some_and(|custody| {
            custody
                .settle(
                    Duration::ZERO,
                    process_discovery_force_grace(
                        PROCESS_SETTLEMENT_EPOCH_GRACE,
                        PROCESS_POLL_INTERVAL,
                    ),
                    PROCESS_POLL_INTERVAL,
                )
                .is_ok()
        });
        let reaped = self.reap_before(self.drop_reap_grace()).unwrap_or(false);
        let group_empty = self.group_settled
            || has_other_members(self.pid, self.pid).is_ok_and(|present| !present);
        let mut streams = Streams::default();
        let output_closed = streams.drain(self).is_ok() && streams.complete();
        if discovery_settled && reaped && group_empty && output_closed {
            self.settle();
        }
    }
}
