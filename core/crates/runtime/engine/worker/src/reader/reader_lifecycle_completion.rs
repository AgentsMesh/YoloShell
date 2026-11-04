use super::{ReaderLifecycle, ReaderProgress};

impl ReaderProgress {
    pub(crate) fn is_drained(&self) -> bool {
        (self.eof.is_observed() || self.read_stopped)
            && !self.active
            && !self.output_read_in_flight
            && self.pending.is_empty()
    }

    pub(crate) fn has_reader_supervisor_completion(&self) -> bool {
        self.reader_supervisor_completion.is_some()
    }
}

impl ReaderLifecycle {
    pub(crate) fn master_aliases_can_close(&self) -> bool {
        let progress = self.lock_progress();
        (progress.eof.is_observed() || progress.output_abandoned)
            && progress.reader_supervisor_completion.is_some()
            && !progress.active
            && !progress.output_read_in_flight
            && progress.pending.is_empty()
    }

    pub(crate) fn mark_tracked_producers_gone(
        &self,
        _proof: &crate::process::TrackedProducersGone,
    ) {
        self.mark_producers_gone();
    }

    pub(crate) fn mark_untracked_producers_gone(
        &self,
        _proof: crate::worker::startup::startup_untracked_proof::UntrackedStartupProducersGone,
    ) {
        self.lock_progress().producers_gone = true;
        self.changed.notify_all();
    }

    fn mark_producers_gone(&self) {
        self.lock_progress().producers_gone = true;
        self.changed.notify_all();
    }
}
