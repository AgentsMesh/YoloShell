use super::{PendingOutput, ReaderLifecycle};
use crate::worker::PTY_PENDING_OUTPUT_CONFLICT_DETAIL;
use crate::worker::pty_output_progress::{KernelPtyEofIdentity, OutputQuiescenceState};
use anyhow::{Context, Result};
use std::sync::Arc;
use std::sync::atomic::Ordering;

const OUTPUT_PROBE_EPOCH_INCREMENT: u64 = 1;
const OUTPUT_PROBE_EPOCH_EXHAUSTED_DETAIL: &str = "PTY output probe epoch is exhausted";
const OUTPUT_READ_ALREADY_IN_FLIGHT_DETAIL: &str = "PTY output read is already in flight";
const OUTPUT_READ_CANCELLED_DETAIL: &str = "PTY output read was cancelled before admission";
const OUTPUT_DISCARD_REQUIRES_NAMESPACE_LOSS_DETAIL: &str =
    "PTY output can only be discarded after namespace loss";

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct OutputProbeEpoch(u64);

pub(crate) struct OutputReadAttempt<'a> {
    pub(crate) lifecycle: &'a ReaderLifecycle,
    pub(crate) epoch: OutputProbeEpoch,
    pub(crate) kernel_eof_probe: Option<Arc<KernelPtyEofIdentity>>,
    pub(crate) completed: bool,
}

impl OutputProbeEpoch {
    fn next(self) -> Result<Self> {
        self.0
            .checked_add(OUTPUT_PROBE_EPOCH_INCREMENT)
            .map(Self)
            .context(OUTPUT_PROBE_EPOCH_EXHAUSTED_DETAIL)
    }

    #[cfg(test)]
    pub(crate) const fn maximum_for_test() -> Self {
        Self(u64::MAX)
    }
}

impl ReaderLifecycle {
    pub(crate) fn request_output_probe(&self) -> Result<OutputProbeEpoch> {
        let mut progress = self.lock_progress();
        let epoch = progress.requested_probe_epoch.next()?;
        progress.requested_probe_epoch = epoch;
        if progress.kernel_eof_probe_identity.is_none() {
            self.output_reader_wakeup.store(true, Ordering::Release);
        }
        self.changed.notify_all();
        Ok(epoch)
    }

    pub(crate) fn begin_output_read(&self) -> Result<OutputReadAttempt<'_>> {
        let mut progress = self.lock_progress();
        anyhow::ensure!(
            !self.is_cancelled() && !progress.read_stopped,
            OUTPUT_READ_CANCELLED_DETAIL
        );
        anyhow::ensure!(
            !progress.output_read_in_flight,
            OUTPUT_READ_ALREADY_IN_FLIGHT_DETAIL
        );
        let kernel_eof_probe = progress.kernel_eof_probe_identity.clone();
        progress.eof.clear();
        progress.kernel_eof_seal_epoch = None;
        progress.output_read_in_flight = true;
        self.output_reader_wakeup.store(false, Ordering::Release);
        Ok(OutputReadAttempt {
            lifecycle: self,
            epoch: progress.requested_probe_epoch,
            kernel_eof_probe,
            completed: false,
        })
    }

    pub(crate) fn output_quiescence_state(&self, epoch: OutputProbeEpoch) -> OutputQuiescenceState {
        let progress = self.lock_progress();
        if !progress.pending.is_empty() {
            OutputQuiescenceState::Pending
        } else if progress.eof.is_awaiting_publication() {
            OutputQuiescenceState::Reached
        } else if progress.output_read_in_flight {
            OutputQuiescenceState::Pending
        } else if progress.eof.has_authority()
            || progress.eof.is_observed()
            || progress.kernel_eof_seal_epoch.is_some()
        {
            OutputQuiescenceState::Reached
        } else if progress.kernel_eof_probe_identity.is_some() {
            OutputQuiescenceState::Reached
        } else if progress
            .drained_probe_epoch
            .is_some_and(|drained| drained >= epoch)
        {
            OutputQuiescenceState::Reached
        } else {
            OutputQuiescenceState::Pending
        }
    }
}

impl<'a> OutputReadAttempt<'a> {
    pub(crate) fn stage(self, data: &[u8], attempt: crate::journal::AppendAttempt) -> Result<()> {
        self.stage_inner(data, Some(attempt))
    }

    pub(crate) fn stage_without_attempt(self, data: &[u8]) -> Result<()> {
        self.stage_inner(data, None)
    }

    #[cfg(test)]
    pub(crate) fn stage_for_test(self, data: &[u8]) -> Result<()> {
        self.stage_without_attempt(data)
    }

    fn stage_inner(
        mut self,
        data: &[u8],
        attempt: Option<crate::journal::AppendAttempt>,
    ) -> Result<()> {
        let mut progress = self.lifecycle.lock_progress();
        progress.drained_probe_epoch = None;
        progress.eof.clear();
        progress.kernel_eof_seal_epoch = None;
        progress.pending.push(PendingOutput {
            data: data.to_vec(),
            attempt,
        });
        progress.pending_commit_retries = super::super::PendingCommitRetryBudget::default();
        self.complete_locked(&mut progress);
        Ok(())
    }

    pub(crate) fn mark_quiescent(mut self) -> Result<()> {
        let mut progress = self.lifecycle.lock_progress();
        anyhow::ensure!(
            progress.pending.is_empty(),
            PTY_PENDING_OUTPUT_CONFLICT_DETAIL
        );
        progress.drained_probe_epoch = Some(
            progress
                .drained_probe_epoch
                .map_or(self.epoch, |drained| drained.max(self.epoch)),
        );
        self.complete_locked(&mut progress);
        Ok(())
    }

    pub(crate) fn discard_after_namespace_loss(mut self) -> Result<()> {
        let mut progress = self.lifecycle.lock_progress();
        anyhow::ensure!(
            progress.namespace_loss_drain == super::namespace_loss::NamespaceLossDrainState::Active,
            OUTPUT_DISCARD_REQUIRES_NAMESPACE_LOSS_DETAIL
        );
        anyhow::ensure!(
            progress.pending.is_empty(),
            PTY_PENDING_OUTPUT_CONFLICT_DETAIL
        );
        progress.drained_probe_epoch = None;
        progress.eof.clear();
        progress.kernel_eof_seal_epoch = None;
        self.complete_locked(&mut progress);
        Ok(())
    }

    fn complete_locked(&mut self, progress: &mut super::ReaderProgress) {
        if let Some(identity) = self.kernel_eof_probe.as_ref() {
            self.lifecycle
                .revoke_kernel_eof_probe_locked(progress, Some(identity));
        }
        progress.output_read_in_flight = false;
        self.completed = true;
        self.lifecycle.changed.notify_all();
    }
}

impl Drop for OutputReadAttempt<'_> {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        let mut progress = self.lifecycle.lock_progress();
        if let Some(identity) = self.kernel_eof_probe.as_ref() {
            self.lifecycle
                .revoke_kernel_eof_probe_locked(&mut progress, Some(identity));
        }
        progress.output_read_in_flight = false;
        self.lifecycle.changed.notify_all();
    }
}
