use super::*;

#[derive(Default)]
pub(super) struct AutomaticResumeTestProbes {
    pub(super) after_crash_exit: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

impl Host {
    pub(super) fn run_after_automatic_crash_exit_probe(&self) {
        let probe = lock_recover(&self.automatic_resume_test_probes.after_crash_exit).take();
        if let Some(probe) = probe {
            probe();
        }
    }
}
