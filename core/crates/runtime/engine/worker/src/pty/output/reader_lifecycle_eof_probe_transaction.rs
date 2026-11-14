use super::KernelPtyEofIdentity;
use super::reader_probe::{
    CapturedProbeValidation, capture_probe_validation, identity_is_current,
    kernel_eof_probe_is_ready, revoke_probe,
};
use crate::worker::ReaderLifecycle;
use crate::worker::reader::reader_lifecycle::{KernelEofTransactionDisposition, ReaderProgress};
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::Ordering;

impl ReaderLifecycle {
    pub(crate) fn with_validated_current_kernel_eof_probe(
        &self,
        expected: &Arc<KernelPtyEofIdentity>,
        validate: impl FnOnce() -> Result<bool>,
        operation: impl FnOnce() -> Result<KernelEofTransactionDisposition>,
    ) -> Result<bool> {
        let mut progress = self.lock_progress();
        if !probe_is_usable(self, &progress, expected) {
            revoke_probe(self, &mut progress, Some(expected));
            return Ok(false);
        }
        match capture_probe_validation(validate) {
            CapturedProbeValidation::Valid(true) => {}
            CapturedProbeValidation::Valid(false) => {
                revoke_probe(self, &mut progress, Some(expected));
                return Ok(false);
            }
            CapturedProbeValidation::Error(error) => {
                revoke_probe(self, &mut progress, Some(expected));
                return Err(error);
            }
            CapturedProbeValidation::Panic(payload) => {
                revoke_probe(self, &mut progress, Some(expected));
                drop(progress);
                std::panic::resume_unwind(payload)
            }
        }
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
        match outcome {
            Ok(Ok(disposition)) => Ok(apply_probe_disposition(
                self,
                &mut progress,
                expected,
                disposition,
            )),
            Ok(Err(error)) => {
                retain_probe(self, &mut progress, expected);
                Err(error)
            }
            Err(payload) => {
                retain_probe(self, &mut progress, expected);
                drop(progress);
                std::panic::resume_unwind(payload)
            }
        }
    }
}

pub(crate) fn probe_is_usable(
    lifecycle: &ReaderLifecycle,
    progress: &ReaderProgress,
    expected: &Arc<KernelPtyEofIdentity>,
) -> bool {
    identity_is_current(&progress.kernel_eof_probe_identity, expected)
        && kernel_eof_probe_is_ready(lifecycle, progress)
}

pub(crate) fn apply_probe_disposition(
    lifecycle: &ReaderLifecycle,
    progress: &mut ReaderProgress,
    expected: &Arc<KernelPtyEofIdentity>,
    disposition: KernelEofTransactionDisposition,
) -> bool {
    match disposition {
        KernelEofTransactionDisposition::Complete => {
            progress.kernel_eof_probe_identity = None;
            progress.kernel_eof_seal_epoch = None;
            progress.eof.observe(Arc::clone(expected));
            lifecycle.changed.notify_all();
            true
        }
        KernelEofTransactionDisposition::Preserve => {
            retain_probe(lifecycle, progress, expected);
            true
        }
        KernelEofTransactionDisposition::PreserveSealed(epoch) => {
            if !epoch.is_current(expected)
                || progress
                    .kernel_eof_seal_epoch
                    .as_ref()
                    .is_some_and(|current| !current.same_authority(&epoch))
            {
                revoke_probe(lifecycle, progress, Some(expected));
                return false;
            }
            retain_probe(lifecycle, progress, expected);
            progress.kernel_eof_seal_epoch = Some(epoch);
            true
        }
        KernelEofTransactionDisposition::Revoke => {
            revoke_probe(lifecycle, progress, Some(expected));
            false
        }
    }
}

pub(crate) fn retain_probe(
    lifecycle: &ReaderLifecycle,
    progress: &mut ReaderProgress,
    expected: &Arc<KernelPtyEofIdentity>,
) {
    if !identity_is_current(&progress.kernel_eof_probe_identity, expected) {
        return;
    }
    progress.kernel_eof_probe_identity = None;
    progress.eof.retain_observation(Arc::clone(expected));
    lifecycle
        .output_reader_wakeup
        .store(true, Ordering::Release);
    lifecycle.changed.notify_all();
}
