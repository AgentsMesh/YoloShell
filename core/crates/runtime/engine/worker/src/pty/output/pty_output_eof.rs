use super::{OutputDescriptor, PtyOutputRecovery};
use crate::process::ProcessClosureVersion;
use anyhow::Result;

#[path = "pty_output_eof_authority.rs"]
mod authority;
#[path = "pty_output_eof_commit.rs"]
mod commit;
#[path = "pty_output_eof_exclusive.rs"]
mod exclusive;
#[path = "pty_output_eof_seal_epoch.rs"]
mod seal_epoch;
#[path = "pty_output_eof_validation.rs"]
mod validation;
pub(crate) use commit::{KernelEofCommitOutcome, KernelEofValidation, KernelEofValidationRequest};
pub(crate) use seal_epoch::CommittedKernelEofSealEpoch;
#[path = "reader_lifecycle_eof.rs"]
mod reader;
#[path = "reader_lifecycle_eof_adoption.rs"]
mod reader_adoption;
#[path = "reader_lifecycle_eof_probe.rs"]
mod reader_probe;
#[path = "reader_lifecycle_eof_probe_transaction.rs"]
mod reader_probe_transaction;
#[path = "reader_lifecycle_eof_publication.rs"]
mod reader_publication;
#[path = "reader_lifecycle_eof_transaction.rs"]
mod reader_transaction;
pub(crate) use authority::{KernelPtyEofAuthority, KernelPtyEofIdentity};

impl PtyOutputRecovery {
    pub(crate) fn acquire_kernel_eof_authority(
        &mut self,
        version: &ProcessClosureVersion,
    ) -> Result<Option<KernelPtyEofAuthority>> {
        let descriptor = self.descriptor.lock();
        let (OutputDescriptor::Native(file), Some(lifecycle), Some(binding)) = (
            &*descriptor,
            self.reader_lifecycle.as_ref(),
            self.master_alias_authority.as_ref(),
        ) else {
            return Ok(None);
        };
        if let Some(authority) = KernelPtyEofAuthority::reader(version, binding, lifecycle) {
            return Ok(Some(authority));
        }
        if !version.binds_pty_output(binding) {
            return Ok(None);
        }
        let identity = lifecycle.acquire_kernel_eof_probe(|| {
            super::super::pty_output_terminal::validates_kernel_pty_eof(file)
        })?;
        Ok(identity.as_ref().and_then(|identity| {
            KernelPtyEofAuthority::reader_probe(version, binding, lifecycle, identity)
        }))
    }

    pub(crate) fn validates_kernel_eof_authority(
        &mut self,
        authority: &KernelPtyEofAuthority,
        version: &ProcessClosureVersion,
    ) -> Result<bool> {
        Ok(
            match (
                &*self.descriptor.lock(),
                self.reader_lifecycle.as_ref(),
                self.master_alias_authority.as_ref(),
            ) {
                (OutputDescriptor::Native(file), Some(lifecycle), Some(binding)) => {
                    if let Some(identity) = authority.reader_identity(version, binding, lifecycle) {
                        return lifecycle.validates_current_kernel_eof(identity, || {
                            super::super::pty_output_terminal::validates_consumed_kernel_pty_eof(
                                file,
                            )
                        });
                    }
                    if let Some(identity) =
                        authority.reader_probe_identity(version, binding, lifecycle)
                    {
                        return lifecycle.validates_current_kernel_eof_probe(identity, || {
                            super::super::pty_output_terminal::validates_kernel_pty_eof(file)
                        });
                    }
                    return Ok(false);
                }
                _ => false,
            },
        )
    }
}

impl super::ReaderBackedPtyOutput {
    pub(crate) fn supports_tracked_settlement(&self) -> bool {
        match &*self.descriptor.lock() {
            OutputDescriptor::Native(_) => {
                self.reader_lifecycle.is_some()
                    && self.master_alias_authority.is_some()
                    && self.descriptor.runtime.is_some()
            }
            OutputDescriptor::Released => {
                self.reader_lifecycle.is_some()
                    && self.master_alias_authority.is_some()
                    && self.descriptor.runtime.is_some()
            }
            #[cfg(test)]
            OutputDescriptor::Observed(_) | OutputDescriptor::Quiescent(_) => false,
        }
    }
}
