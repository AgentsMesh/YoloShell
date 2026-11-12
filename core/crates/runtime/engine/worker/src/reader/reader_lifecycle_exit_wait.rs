use super::ReaderLifecycle;
use std::sync::atomic::Ordering;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReaderExitConfirmation {
    ChildExited,
    Cancelled,
    OutputProbe,
}

impl ReaderLifecycle {
    pub(crate) fn wait_for_child_exit_confirmation(
        &self,
        confirmed: &std::sync::atomic::AtomicBool,
    ) -> ReaderExitConfirmation {
        let mut progress = self.lock_progress();
        while !confirmed.load(Ordering::Acquire)
            && !progress.producers_gone
            && !self.is_cancelled()
            && !self.output_reader_wakeup.load(Ordering::Acquire)
        {
            progress = self
                .changed
                .wait(progress)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        if confirmed.load(Ordering::Acquire) || progress.producers_gone {
            ReaderExitConfirmation::ChildExited
        } else if self.is_cancelled() {
            ReaderExitConfirmation::Cancelled
        } else {
            ReaderExitConfirmation::OutputProbe
        }
    }

    pub(crate) fn wake_output_reader(&self) {
        let progress = self.lock_progress();
        if progress.kernel_eof_probe_identity.is_none() {
            self.output_reader_wakeup.store(true, Ordering::Release);
        }
        self.changed.notify_all();
    }
}
