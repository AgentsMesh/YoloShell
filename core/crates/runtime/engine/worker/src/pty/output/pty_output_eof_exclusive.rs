use super::{CommittedKernelEofSealEpoch, KernelPtyEofAuthority, KernelPtyEofIdentity};
use crate::process::ProcessClosureVersion;
use crate::worker::pty_output_progress::{
    ExclusivePtyOutput, OutputDescriptor, OutputProgressState,
};
use anyhow::Result;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

impl ExclusivePtyOutput {
    pub(crate) fn accepts_eof_handoff(&self) -> bool {
        !self.output_abandoned
    }

    pub(crate) fn supports_tracked_settlement(&self) -> bool {
        match &self.descriptor {
            OutputDescriptor::Native(_) => self.master_alias_authority.is_some(),
            OutputDescriptor::Released => {
                self.owner_master_released && self.master_alias_authority.is_some()
            }
            #[cfg(test)]
            OutputDescriptor::Observed(_) | OutputDescriptor::Quiescent(_) => false,
        }
    }

    pub(crate) fn acquire_kernel_eof_authority(
        &mut self,
        version: &ProcessClosureVersion,
    ) -> Result<Option<KernelPtyEofAuthority>> {
        let progress = self.progress()?;
        if progress == OutputProgressState::Abandoned {
            return Ok(None);
        }
        let authority = match (
            &self.descriptor,
            self.current_kernel_eof_identity(),
            self.master_alias_authority.as_ref(),
        ) {
            (OutputDescriptor::Native(_), Some(identity), Some(binding)) => {
                KernelPtyEofAuthority::exclusive(
                    version,
                    binding,
                    identity,
                    self.kernel_eof_seal_epoch.as_ref(),
                )
            }
            _ => None,
        };
        Ok(authority)
    }

    pub(crate) fn validates_kernel_eof_authority(
        &mut self,
        authority: &KernelPtyEofAuthority,
        version: &ProcessClosureVersion,
    ) -> Result<bool> {
        let (OutputDescriptor::Native(file), Some(binding)) =
            (&self.descriptor, self.master_alias_authority.as_ref())
        else {
            return Ok(false);
        };
        if !authority.validates_exclusive(
            version,
            binding,
            self.current_kernel_eof_identity(),
            self.kernel_eof_seal_epoch.as_ref(),
        ) {
            return Ok(false);
        }
        let current = catch_unwind(AssertUnwindSafe(|| {
            crate::worker::pty_output_terminal::validates_kernel_pty_eof(file)
        }));
        match current {
            Ok(Ok(true)) => Ok(true),
            Ok(Ok(false)) => {
                authority.invalidate_output();
                self.revoke_kernel_eof_authority();
                Ok(false)
            }
            Ok(Err(error)) => {
                authority.invalidate_output();
                self.revoke_kernel_eof_authority();
                Err(error)
            }
            Err(payload) => {
                authority.invalidate_output();
                self.revoke_kernel_eof_authority();
                std::panic::resume_unwind(payload)
            }
        }
    }

    pub(crate) fn revoke_kernel_eof_authority(&mut self) {
        self.kernel_eof = None;
        self.kernel_eof_seal_epoch = None;
        self.kernel_eof_committed = None;
    }

    pub(crate) fn current_kernel_eof_identity(&self) -> Option<&Arc<KernelPtyEofIdentity>> {
        self.kernel_eof_seal_epoch
            .as_ref()
            .map(CommittedKernelEofSealEpoch::identity)
            .or(self.kernel_eof.as_ref())
    }

    pub(crate) fn record_kernel_eof_progress(&mut self, progress: OutputProgressState) {
        if self.kernel_eof.is_none() && self.kernel_eof_seal_epoch.is_none() {
            self.kernel_eof = kernel_eof_identity_from_read(progress);
        }
        match progress {
            OutputProgressState::Complete | OutputProgressState::Quiescent => {}
            OutputProgressState::Pending => {
                self.revoke_kernel_eof_authority();
            }
            OutputProgressState::Abandoned => {
                self.revoke_kernel_eof_authority();
            }
        }
    }
}

pub(crate) fn kernel_eof_identity_from_read(
    progress: OutputProgressState,
) -> Option<Arc<KernelPtyEofIdentity>> {
    (progress == OutputProgressState::Complete).then(KernelPtyEofIdentity::from_kernel_read)
}

#[cfg(test)]
impl ExclusivePtyOutput {
    pub(crate) fn abandon_for_activation_test(&mut self) {
        self.output_abandoned = true;
    }
}
