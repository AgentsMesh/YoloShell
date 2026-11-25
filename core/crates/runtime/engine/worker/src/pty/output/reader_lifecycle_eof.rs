use super::KernelPtyEofIdentity;
use super::reader_transaction::reader_eof_is_usable;
use crate::worker::ReaderLifecycle;
use anyhow::Result;
use std::sync::Arc;

impl ReaderLifecycle {
    pub(crate) fn kernel_eof_identity(&self) -> Option<Arc<KernelPtyEofIdentity>> {
        self.kernel_eof_observation()
            .map(|(identity, _seal)| identity)
    }

    pub(crate) fn kernel_eof_observation(
        &self,
    ) -> Option<(
        Arc<KernelPtyEofIdentity>,
        Option<crate::process::CommittedPtyReopenSealAuthority>,
    )> {
        let progress = self.lock_progress();
        if !reader_eof_is_usable(self, &progress) {
            return None;
        }
        if let Some(epoch) = progress.kernel_eof_seal_epoch.as_ref() {
            return Some((Arc::clone(epoch.identity()), Some(epoch.seal().clone())));
        }
        progress
            .eof
            .identity()
            .cloned()
            .map(|identity| (identity, None))
    }

    pub(crate) fn validates_kernel_eof_identity(
        &self,
        expected: &Arc<KernelPtyEofIdentity>,
    ) -> bool {
        let progress = self.lock_progress();
        reader_eof_is_usable(self, &progress)
            && (progress.eof.is_current(expected)
                || progress
                    .kernel_eof_seal_epoch
                    .as_ref()
                    .is_some_and(|epoch| epoch.is_current(expected)))
    }

    pub(crate) fn validates_current_kernel_eof<Validate>(
        &self,
        expected: &Arc<KernelPtyEofIdentity>,
        validate: Validate,
    ) -> Result<bool>
    where
        Validate: FnOnce() -> Result<bool>,
    {
        self.with_validated_current_kernel_eof(expected, validate, || {
            Ok(crate::worker::reader::reader_lifecycle::KernelEofTransactionDisposition::Preserve)
        })
    }

    #[cfg(test)]
    pub(crate) fn mark_eof_observed(&self) {
        let mut progress = self.lock_progress();
        progress
            .eof
            .observe(KernelPtyEofIdentity::from_kernel_read());
        progress.kernel_eof_probe_identity = None;
        progress.kernel_eof_seal_epoch = None;
        self.changed.notify_all();
    }

    #[cfg(test)]
    pub(crate) fn revoke_kernel_eof_for_test(&self) {
        let mut progress = self.lock_progress();
        progress.eof.clear();
        progress.kernel_eof_probe_identity = None;
        progress.kernel_eof_seal_epoch = None;
        self.changed.notify_all();
    }
}
