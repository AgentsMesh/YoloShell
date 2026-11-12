use super::FaultReap;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(crate) const FAULT_REAPER_PANIC_DETAIL: &str = "PTY fault reaper panicked";
pub(crate) const FAULT_REAPER_EARLY_RETURN_DETAIL: &str =
    "PTY fault reaper returned before settling its custody";

impl FaultReap {
    pub(crate) fn reap(mut self) {
        self.reap_retained();
    }

    pub(crate) fn reap_retained(&mut self) {
        self.reap_retained_with_driver(super::live_authority::reap);
    }

    pub(crate) fn reap_with_driver<Driver>(mut self, driver: Driver)
    where
        Driver: FnMut(&mut Self),
    {
        self.reap_retained_with_driver(driver);
    }

    pub(crate) fn reap_retained_with_driver<Driver>(&mut self, mut driver: Driver)
    where
        Driver: FnMut(&mut Self),
    {
        let mut delay = super::constants::PTY_FAULT_REAPER_RETRY_INITIAL_INTERVAL;
        loop {
            let previous = self.progress();
            let result = catch_unwind(AssertUnwindSafe(|| driver(self)));
            match result {
                Ok(()) if !self.has_unsettled_custody() => return,
                Ok(()) => self.remember(Err(anyhow::anyhow!(FAULT_REAPER_EARLY_RETURN_DETAIL))),
                Err(payload) => {
                    let detail = crate::worker::supervision::panic_detail(payload);
                    self.remember(Err(anyhow::anyhow!(
                        "{FAULT_REAPER_PANIC_DETAIL}: {detail}"
                    )));
                }
            }
            if !self.has_unsettled_custody() {
                return;
            }
            let activity = self.take_progress_after(previous);
            if activity == super::schedule::FaultReapActivity::SemanticProgress {
                self.publish_cleanup_progress();
            }
            delay = self.schedule.pause(
                delay,
                activity,
                super::constants::PTY_FAULT_REAPER_RETRY_INITIAL_INTERVAL,
                super::constants::PTY_FAULT_REAPER_RETRY_MAX_INTERVAL,
            );
        }
    }
}
