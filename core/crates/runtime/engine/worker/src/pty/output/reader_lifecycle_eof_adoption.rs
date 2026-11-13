use super::{CommittedKernelEofSealEpoch, KernelPtyEofIdentity};
use crate::worker::ReaderLifecycle;
use crate::worker::pty_output_progress::ExclusivePtyOutput;
use crate::worker::reader::reader_lifecycle::ReaderProgress;
use anyhow::{Result, ensure};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;
use std::sync::atomic::Ordering;

pub(crate) const EOF_HANDOFF_PRECONDITION_CONFLICT_DETAIL: &str =
    "exclusive PTY EOF handoff requires a live source output and unused reader lifecycle";
pub(crate) const EOF_HANDOFF_STATE_CONFLICT_DETAIL: &str =
    "exclusive PTY output contains conflicting EOF authority states";

#[derive(Clone)]
pub(crate) enum ExclusiveKernelEofState {
    Open,
    Raw(Arc<KernelPtyEofIdentity>),
    Sealed(CommittedKernelEofSealEpoch),
    Committed(Arc<KernelPtyEofIdentity>),
}

impl ReaderLifecycle {
    pub(crate) fn adopt_exclusive_kernel_eof_before<Activate>(
        &self,
        output: &mut ExclusivePtyOutput,
        activate: Activate,
    ) -> Result<()>
    where
        Activate: FnOnce() -> Result<()>,
    {
        let mut progress = self.lock_progress();
        ensure!(
            lifecycle_accepts_handoff(self, &progress, output),
            EOF_HANDOFF_PRECONDITION_CONFLICT_DETAIL
        );
        let state = take_exclusive_eof(output)?;
        let rollback = state.clone();
        let previous_wakeup = self.output_reader_wakeup.load(Ordering::Acquire);
        install_reader_eof(self, &mut progress, state);
        match catch_unwind(AssertUnwindSafe(activate)) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                rollback_reader_eof(self, &mut progress, output, rollback, previous_wakeup);
                return Err(error);
            }
            Err(payload) => {
                rollback_reader_eof(self, &mut progress, output, rollback, previous_wakeup);
                drop(progress);
                resume_unwind(payload);
            }
        }
        self.changed.notify_all();
        Ok(())
    }
}

pub(crate) fn lifecycle_accepts_handoff(
    lifecycle: &ReaderLifecycle,
    progress: &ReaderProgress,
    output: &ExclusivePtyOutput,
) -> bool {
    output.accepts_eof_handoff()
        && !lifecycle.is_cancelled()
        && !progress.active
        && !progress.read_stopped
        && !progress.producers_gone
        && !progress.has_reader_supervisor_completion()
        && !progress.output_read_in_flight
        && !progress.output_abandoned
        && progress.pending.is_empty()
        && progress.kernel_eof_probe_identity.is_none()
        && progress.kernel_eof_seal_epoch.is_none()
        && !progress.eof.has_authority()
        && !progress.eof.is_observed()
        && progress.namespace_loss_drain
            == crate::worker::reader::reader_lifecycle::namespace_loss::NamespaceLossDrainState::Inactive
        && progress.requested_probe_epoch == Default::default()
        && progress.drained_probe_epoch.is_none()
}

pub(crate) fn rollback_reader_eof(
    lifecycle: &ReaderLifecycle,
    progress: &mut ReaderProgress,
    output: &mut ExclusivePtyOutput,
    rollback: ExclusiveKernelEofState,
    previous_wakeup: bool,
) {
    clear_reader_eof(progress);
    restore_exclusive_eof(output, rollback);
    lifecycle
        .output_reader_wakeup
        .store(previous_wakeup, Ordering::Release);
}

pub(crate) fn take_exclusive_eof(
    output: &mut ExclusivePtyOutput,
) -> Result<ExclusiveKernelEofState> {
    let occupancy = (
        output.kernel_eof.is_some(),
        output.kernel_eof_seal_epoch.is_some(),
        output.kernel_eof_committed.is_some(),
    );
    ensure!(
        matches!(
            occupancy,
            (false, false, false)
                | (true, false, false)
                | (false, true, false)
                | (false, false, true)
        ),
        EOF_HANDOFF_STATE_CONFLICT_DETAIL
    );
    Ok(if let Some(identity) = output.kernel_eof.take() {
        ExclusiveKernelEofState::Raw(identity)
    } else if let Some(epoch) = output.kernel_eof_seal_epoch.take() {
        ExclusiveKernelEofState::Sealed(epoch)
    } else if let Some(identity) = output.kernel_eof_committed.take() {
        ExclusiveKernelEofState::Committed(identity)
    } else {
        ExclusiveKernelEofState::Open
    })
}

pub(crate) fn install_reader_eof(
    lifecycle: &ReaderLifecycle,
    progress: &mut ReaderProgress,
    state: ExclusiveKernelEofState,
) {
    match state {
        ExclusiveKernelEofState::Open => {}
        ExclusiveKernelEofState::Raw(identity) => progress.eof.retain_observation(identity),
        ExclusiveKernelEofState::Sealed(epoch) => {
            progress.kernel_eof_seal_epoch = Some(epoch);
            lifecycle
                .output_reader_wakeup
                .store(true, Ordering::Release);
        }
        ExclusiveKernelEofState::Committed(identity) => progress.eof.observe(identity),
    }
}

pub(crate) fn clear_reader_eof(progress: &mut ReaderProgress) {
    progress.eof.clear();
    progress.kernel_eof_seal_epoch = None;
}

pub(crate) fn restore_exclusive_eof(
    output: &mut ExclusivePtyOutput,
    state: ExclusiveKernelEofState,
) {
    match state {
        ExclusiveKernelEofState::Open => {}
        ExclusiveKernelEofState::Raw(identity) => output.kernel_eof = Some(identity),
        ExclusiveKernelEofState::Sealed(epoch) => output.kernel_eof_seal_epoch = Some(epoch),
        ExclusiveKernelEofState::Committed(identity) => {
            output.kernel_eof_committed = Some(identity);
        }
    }
}

#[cfg(test)]
impl ExclusivePtyOutput {
    pub(crate) fn install_raw_eof_for_activation_test(&mut self) -> Arc<KernelPtyEofIdentity> {
        let identity = KernelPtyEofIdentity::from_kernel_read();
        self.kernel_eof = Some(Arc::clone(&identity));
        identity
    }

    pub(crate) fn install_sealed_eof_for_activation_test(&mut self) -> Arc<KernelPtyEofIdentity> {
        let identity = KernelPtyEofIdentity::from_kernel_read();
        let seal = crate::process::CommittedPtyReopenSealAuthority::missing_proof_for_test();
        self.kernel_eof_seal_epoch = Some(CommittedKernelEofSealEpoch::new(
            Arc::clone(&identity),
            seal,
        ));
        identity
    }

    pub(crate) fn install_committed_eof_for_activation_test(
        &mut self,
    ) -> Arc<KernelPtyEofIdentity> {
        let identity = KernelPtyEofIdentity::from_kernel_read();
        self.kernel_eof_committed = Some(Arc::clone(&identity));
        identity
    }
}
