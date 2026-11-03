use crate::worker::ReaderLifecycle;
use anyhow::Result;
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use super::KernelPtyEofIdentity;

impl ReaderLifecycle {
    pub(crate) fn acquire_kernel_eof_probe(
        &self,
        validate: impl FnOnce() -> Result<bool>,
    ) -> Result<Option<Arc<KernelPtyEofIdentity>>> {
        let mut progress = self.lock_progress();
        match kernel_eof_probe_state(self, &progress) {
            KernelEofProbeState::Ready => {}
            KernelEofProbeState::ReadInFlight => return Ok(None),
            KernelEofProbeState::Invalid => {
                revoke_probe(self, &mut progress, None);
                return Ok(None);
            }
        }
        let valid = match capture_probe_validation(validate) {
            CapturedProbeValidation::Valid(valid) => valid,
            CapturedProbeValidation::Error(error) => {
                revoke_probe(self, &mut progress, None);
                return Err(error);
            }
            CapturedProbeValidation::Panic(payload) => {
                revoke_probe(self, &mut progress, None);
                drop(progress);
                std::panic::resume_unwind(payload)
            }
        };
        if !valid {
            revoke_probe(self, &mut progress, None);
            return Ok(None);
        }
        let identity = progress
            .kernel_eof_probe_identity
            .get_or_insert_with(KernelPtyEofIdentity::from_kernel_read);
        self.output_reader_wakeup
            .store(false, std::sync::atomic::Ordering::Release);
        Ok(Some(Arc::clone(identity)))
    }

    pub(crate) fn validates_current_kernel_eof_probe(
        &self,
        expected: &Arc<KernelPtyEofIdentity>,
        validate: impl FnOnce() -> Result<bool>,
    ) -> Result<bool> {
        let mut progress = self.lock_progress();
        if !identity_is_current(&progress.kernel_eof_probe_identity, expected) {
            revoke_probe(self, &mut progress, Some(expected));
            return Ok(false);
        }
        match kernel_eof_probe_state(self, &progress) {
            KernelEofProbeState::Ready => {}
            KernelEofProbeState::ReadInFlight => return Ok(false),
            KernelEofProbeState::Invalid => {
                revoke_probe(self, &mut progress, Some(expected));
                return Ok(false);
            }
        }
        match capture_probe_validation(validate) {
            CapturedProbeValidation::Valid(true) => Ok(true),
            CapturedProbeValidation::Valid(false) => {
                revoke_probe(self, &mut progress, Some(expected));
                Ok(false)
            }
            CapturedProbeValidation::Error(error) => {
                revoke_probe(self, &mut progress, Some(expected));
                Err(error)
            }
            CapturedProbeValidation::Panic(payload) => {
                revoke_probe(self, &mut progress, Some(expected));
                drop(progress);
                std::panic::resume_unwind(payload)
            }
        }
    }

    pub(crate) fn revoke_kernel_eof_probe_locked(
        &self,
        progress: &mut crate::worker::reader::reader_lifecycle::ReaderProgress,
        expected: Option<&Arc<KernelPtyEofIdentity>>,
    ) {
        revoke_probe(self, progress, expected);
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum KernelEofProbeState {
    Ready,
    ReadInFlight,
    Invalid,
}

pub(crate) enum CapturedProbeValidation {
    Valid(bool),
    Error(anyhow::Error),
    Panic(Box<dyn Any + Send>),
}

pub(crate) fn capture_probe_validation(
    validate: impl FnOnce() -> Result<bool>,
) -> CapturedProbeValidation {
    match catch_unwind(AssertUnwindSafe(validate)) {
        Ok(Ok(valid)) => CapturedProbeValidation::Valid(valid),
        Ok(Err(error)) => CapturedProbeValidation::Error(error),
        Err(payload) => CapturedProbeValidation::Panic(payload),
    }
}

pub(crate) fn kernel_eof_probe_state(
    lifecycle: &ReaderLifecycle,
    progress: &crate::worker::reader::reader_lifecycle::ReaderProgress,
) -> KernelEofProbeState {
    let output_authority_available = !progress.output_abandoned
        || progress.namespace_loss_drain
            == crate::worker::reader::reader_lifecycle::namespace_loss::NamespaceLossDrainState::Active;
    let valid = !lifecycle.is_cancelled()
        && output_authority_available
        && !progress.read_stopped
        && !progress.eof.has_authority()
        && !progress.eof.is_observed()
        && progress.kernel_eof_seal_epoch.is_none()
        && progress.pending.is_empty();
    if !valid {
        KernelEofProbeState::Invalid
    } else if progress.output_read_in_flight {
        KernelEofProbeState::ReadInFlight
    } else {
        KernelEofProbeState::Ready
    }
}

pub(crate) fn kernel_eof_probe_is_ready(
    lifecycle: &ReaderLifecycle,
    progress: &crate::worker::reader::reader_lifecycle::ReaderProgress,
) -> bool {
    kernel_eof_probe_state(lifecycle, progress) == KernelEofProbeState::Ready
}

pub(crate) fn revoke_probe(
    lifecycle: &ReaderLifecycle,
    progress: &mut crate::worker::reader::reader_lifecycle::ReaderProgress,
    expected: Option<&Arc<KernelPtyEofIdentity>>,
) {
    let current = progress.kernel_eof_probe_identity.as_ref();
    if current.is_none() {
        return;
    }
    if expected
        .is_some_and(|expected| !current.is_some_and(|current| Arc::ptr_eq(current, expected)))
    {
        return;
    }
    progress.kernel_eof_probe_identity = None;
    if expected.is_some_and(|expected| {
        progress
            .kernel_eof_seal_epoch
            .as_ref()
            .is_some_and(|epoch| epoch.is_current(expected))
    }) {
        progress.kernel_eof_seal_epoch = None;
    }
    lifecycle
        .output_reader_wakeup
        .store(true, std::sync::atomic::Ordering::Release);
    lifecycle.changed.notify_all();
}

pub(crate) fn identity_is_current(
    current: &Option<Arc<KernelPtyEofIdentity>>,
    expected: &Arc<KernelPtyEofIdentity>,
) -> bool {
    current
        .as_ref()
        .is_some_and(|current| Arc::ptr_eq(current, expected))
}
