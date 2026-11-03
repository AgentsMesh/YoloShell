use super::CommittedKernelEofSealEpoch;
use super::KernelPtyEofAuthority;
use crate::process::CommittedPtyReopenSealAuthority;
use crate::process::kernel_eof_authority::{ValidatedKernelEof, ValidatedSealedKernelEof};

impl KernelPtyEofAuthority {
    pub(crate) fn bind_committed_reopen_seal(
        &mut self,
        validated: ValidatedKernelEof,
        seal: &CommittedPtyReopenSealAuthority,
    ) -> Option<ValidatedSealedKernelEof> {
        let sealed = self
            .validation_authority()?
            .bind_committed_reopen_seal(validated, seal)?;
        let epoch = CommittedKernelEofSealEpoch::new(
            std::sync::Arc::clone(self.source.identity()),
            seal.clone(),
        );
        if self
            .seal_epoch
            .as_ref()
            .is_some_and(|current| !current.same_authority(&epoch))
        {
            self.invalidate_output();
            return None;
        }
        self.seal_epoch = Some(epoch);
        Some(sealed)
    }
}
