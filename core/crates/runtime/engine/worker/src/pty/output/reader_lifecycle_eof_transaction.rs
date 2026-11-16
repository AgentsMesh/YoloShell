use super::{CommittedKernelEofSealEpoch, KernelPtyEofIdentity};
use crate::worker::ReaderLifecycle;
use crate::worker::reader::reader_lifecycle::{KernelEofTransactionDisposition, ReaderProgress};
use anyhow::Result;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::Ordering;

impl ReaderLifecycle {
    pub(crate) fn with_validated_current_kernel_eof<Validate, Operation>(
        &self,
        expected: &Arc<KernelPtyEofIdentity>,
        validate: Validate,
        operation: Operation,
    ) -> Result<bool>
    where
        Validate: FnOnce() -> Result<bool>,
        Operation: FnOnce() -> Result<KernelEofTransactionDisposition>,
    {
        let mut progress = self.lock_progress();
        let identity_is_current = progress.eof.is_current(expected)
            || progress
                .kernel_eof_seal_epoch
                .as_ref()
                .is_some_and(|epoch| epoch.is_current(expected));
        if !reader_eof_is_usable(self, &progress) || !identity_is_current {
            return Ok(false);
        }
        match catch_unwind(AssertUnwindSafe(validate)) {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) => {
                revoke_reader_eof(self, &mut progress, expected);
                return Ok(false);
            }
            Ok(Err(error)) => {
                revoke_reader_eof(self, &mut progress, expected);
                return Err(error);
            }
            Err(payload) => {
                revoke_reader_eof(self, &mut progress, expected);
                drop(progress);
                std::panic::resume_unwind(payload)
            }
        }
        let disposition = match catch_unwind(AssertUnwindSafe(operation)) {
            Ok(Ok(disposition)) => disposition,
            Ok(Err(error)) => {
                if seal_epoch_is_current(&progress, expected) {
                    revoke_reader_eof(self, &mut progress, expected);
                }
                return Err(error);
            }
            Err(payload) => {
                if seal_epoch_is_current(&progress, expected) {
                    revoke_reader_eof(self, &mut progress, expected);
                }
                drop(progress);
                std::panic::resume_unwind(payload)
            }
        };
        match disposition {
            KernelEofTransactionDisposition::Complete => {
                progress.eof.observe(Arc::clone(expected));
                progress.kernel_eof_seal_epoch = None;
                progress.output_read_in_flight = false;
                self.changed.notify_all();
                Ok(true)
            }
            KernelEofTransactionDisposition::Preserve => Ok(true),
            KernelEofTransactionDisposition::PreserveSealed(epoch) => {
                if install_seal_epoch(&mut progress, expected, epoch) {
                    self.output_reader_wakeup.store(true, Ordering::Release);
                    self.changed.notify_all();
                    Ok(true)
                } else {
                    revoke_reader_eof(self, &mut progress, expected);
                    Ok(false)
                }
            }
            KernelEofTransactionDisposition::Revoke => {
                revoke_reader_eof(self, &mut progress, expected);
                Ok(false)
            }
        }
    }
}

pub(crate) fn revoke_reader_eof(
    lifecycle: &ReaderLifecycle,
    progress: &mut ReaderProgress,
    expected: &Arc<KernelPtyEofIdentity>,
) {
    progress.eof.revoke_if_current(expected);
    if seal_epoch_is_current(progress, expected) {
        progress.kernel_eof_seal_epoch = None;
    }
    lifecycle
        .output_reader_wakeup
        .store(true, Ordering::Release);
    lifecycle.changed.notify_all();
}

pub(crate) fn install_seal_epoch(
    progress: &mut ReaderProgress,
    expected: &Arc<KernelPtyEofIdentity>,
    epoch: CommittedKernelEofSealEpoch,
) -> bool {
    if !epoch.is_current(expected)
        || progress
            .kernel_eof_seal_epoch
            .as_ref()
            .is_some_and(|current| !current.same_authority(&epoch))
    {
        return false;
    }
    progress.kernel_eof_seal_epoch = Some(epoch);
    true
}

pub(crate) fn seal_epoch_is_current(
    progress: &ReaderProgress,
    expected: &Arc<KernelPtyEofIdentity>,
) -> bool {
    progress
        .kernel_eof_seal_epoch
        .as_ref()
        .is_some_and(|epoch| epoch.is_current(expected))
}

pub(crate) fn reader_eof_is_usable(lifecycle: &ReaderLifecycle, progress: &ReaderProgress) -> bool {
    let output_authority_available = !progress.output_abandoned
        || progress.namespace_loss_drain
            == crate::worker::reader::reader_lifecycle::namespace_loss::NamespaceLossDrainState::Active;
    !lifecycle.is_cancelled()
        && output_authority_available
        && !progress.read_stopped
        && (!progress.output_read_in_flight || progress.eof.is_awaiting_publication())
        && progress.pending.is_empty()
        && (progress.eof.has_authority() || progress.kernel_eof_seal_epoch.is_some())
}
