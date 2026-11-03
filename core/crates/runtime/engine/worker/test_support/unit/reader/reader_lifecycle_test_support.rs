use super::ReaderLifecycle;

impl ReaderLifecycle {
    pub(in crate::worker) fn clear_pending_output_for_test(&self) {
        self.lock_progress().pending.clear();
    }

    pub(in crate::worker) fn exhaust_output_probe_epoch_for_test(&self) {
        self.lock_progress().requested_probe_epoch = super::OutputProbeEpoch::maximum_for_test();
    }

    pub(in crate::worker) fn mark_producers_gone_for_test(&self) {
        self.lock_progress().producers_gone = true;
        self.changed.notify_all();
    }

    pub(in crate::worker) fn mark_drained(&self) {
        let mut progress = self.lock_progress();
        progress.active = false;
        progress.eof.mark_drained();
        progress.kernel_eof_probe_identity = None;
        progress.read_stopped = false;
        progress.pending.clear();
        drop(progress);
        self.mark_supervisor_completed_for_test();
    }

    pub(in crate::worker) fn install_restart_wait_probe(
        &self,
        probe: std::sync::mpsc::SyncSender<()>,
    ) {
        let previous = self
            .restart_wait_probe
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .replace(probe);
        assert!(
            previous.is_none(),
            "{}",
            super::RESTART_WAIT_PROBE_INSTALLED_DETAIL
        );
    }
}
