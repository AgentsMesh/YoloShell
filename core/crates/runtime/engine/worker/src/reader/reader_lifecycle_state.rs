impl ReaderLifecycle {
    pub(crate) fn has_pending(&self) -> bool {
        self.pending_count() != 0
    }

    pub(crate) fn pending_count(&self) -> usize {
        self.progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .pending
            .len()
    }

    pub(crate) fn pending_payload_bytes(&self) -> usize {
        self.progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .pending
            .iter()
            .fold(0_usize, |bytes, pending| {
                bytes.saturating_add(pending.data.len())
            })
    }

    pub(crate) fn begin_attempt(&self) -> bool {
        let mut progress = self.lock_progress();
        if self.is_cancelled() || progress.read_stopped {
            return false;
        }
        progress.active = true;
        self.changed.notify_all();
        true
    }

    pub(crate) fn finish_attempt(&self) {
        self.lock_progress().active = false;
        self.changed.notify_all();
    }

    pub(crate) fn mark_read_stopped(&self) {
        let mut progress = self.lock_progress();
        progress.read_stopped = true;
        progress.namespace_loss_drain = namespace_loss::NamespaceLossDrainState::Inactive;
        progress.eof.clear();
        progress.kernel_eof_probe_identity = None;
        progress.kernel_eof_seal_epoch = None;
        self.changed.notify_all();
    }

    pub(crate) fn wait_for_restart(
        &self,
        timeout: Duration,
        confirmed: &AtomicBool,
        phase: ReaderRestartPhase,
        kind: ReaderRestartKind,
    ) {
        let progress = self.lock_progress();
        if phase.child_exit_interrupts(confirmed)
            || kind.cancellation_interrupts(self.is_cancelled())
        {
            return;
        }
        #[cfg(test)]
        if let Some(probe) = self
            .restart_wait_probe
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            let _ = probe.send(());
        }
        let waited = self.changed.wait_timeout_while(progress, timeout, |_| {
            !phase.child_exit_interrupts(confirmed)
                && !kind.cancellation_interrupts(self.is_cancelled())
        });
        drop(waited.unwrap_or_else(|poisoned| poisoned.into_inner()));
    }

    pub(crate) fn notify_child_exit_confirmation(&self) {
        let progress = self.lock_progress();
        self.changed.notify_all();
        drop(progress);
    }

    pub(crate) fn producers_are_gone(&self) -> bool {
        self.lock_progress().producers_gone
    }

    pub(crate) fn output_progress_state(&self) -> OutputProgressState {
        let progress = self.lock_progress();
        if progress.output_abandoned {
            OutputProgressState::Abandoned
        } else if !progress.pending.is_empty() || progress.output_read_in_flight {
            OutputProgressState::Pending
        } else if progress.kernel_eof_probe_identity.is_some() || progress.eof.is_retained() {
            OutputProgressState::Pending
        } else if progress.eof.is_observed() && !progress.active {
            OutputProgressState::Complete
        } else {
            OutputProgressState::Quiescent
        }
    }

    pub(crate) fn cancel(&self) {
        let mut progress = self.lock_progress();
        self.cancel_locked(&mut progress);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub(crate) fn lock_progress(&self) -> std::sync::MutexGuard<'_, ReaderProgress> {
        self.progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn cancel_locked(&self, progress: &mut ReaderProgress) {
        self.cancelled.store(true, Ordering::Release);
        progress.read_stopped = true;
        progress.namespace_loss_drain = namespace_loss::NamespaceLossDrainState::Inactive;
        progress.eof.clear();
        progress.kernel_eof_probe_identity = None;
        progress.kernel_eof_seal_epoch = None;
        self.changed.notify_all();
    }
}
