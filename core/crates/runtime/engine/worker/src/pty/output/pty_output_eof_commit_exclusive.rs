use super::operation::CapturedKernelEofOperation;
use super::{KernelEofCommitOutcome, KernelEofValidationRequest, Validator};
use crate::process::ProcessClosureVersion;
use crate::worker::pty_output_progress::{
    ExclusivePtyOutput, KernelPtyEofAuthority, OutputDescriptor,
};
use crate::worker::reader::reader_lifecycle::KernelEofTransactionDisposition;
use anyhow::Result;

use super::super::validation::sealed_validation;

impl ExclusivePtyOutput {
    pub(crate) fn commit_if_kernel_eof_current(
        &mut self,
        authority: &mut KernelPtyEofAuthority,
        version: &ProcessClosureVersion,
        operation: impl FnOnce(&mut Validator<'_>) -> Result<KernelEofCommitOutcome>,
    ) -> Result<bool> {
        if !self.validates_kernel_eof_authority(authority, version)? {
            return Ok(false);
        }
        let expected = self.current_kernel_eof_identity().cloned();
        let mut validate = |request| match request {
            KernelEofValidationRequest::Current => {
                if !self.validates_kernel_eof_authority(authority, version)? {
                    return Ok(None);
                }
                Ok(self
                    .master_alias_authority
                    .as_ref()
                    .and_then(|binding| {
                        authority.validate_exclusive(
                            version,
                            binding,
                            self.current_kernel_eof_identity(),
                            self.kernel_eof_seal_epoch.as_ref(),
                        )
                    })
                    .map(super::KernelEofValidation::Current))
            }
            KernelEofValidationRequest::ReopenSealed { validated, seal } => {
                let OutputDescriptor::Native(file) = &self.descriptor else {
                    return Ok(None);
                };
                sealed_validation(file, authority, validated, &seal)
            }
        };
        let captured = CapturedKernelEofOperation::run(|| operation(&mut validate));
        drop(validate);
        let disposition = captured.disposition(authority);
        let accepted = self.apply_kernel_eof_disposition(expected.as_ref(), disposition);
        let outcome = captured.finish()?;
        let committed = accepted && outcome == KernelEofCommitOutcome::Committed;
        if committed || outcome == KernelEofCommitOutcome::OutputInvalidated {
            authority.retire_validation();
        }
        Ok(committed)
    }

    pub(crate) fn apply_kernel_eof_disposition(
        &mut self,
        expected: Option<&std::sync::Arc<super::super::KernelPtyEofIdentity>>,
        disposition: KernelEofTransactionDisposition,
    ) -> bool {
        match disposition {
            KernelEofTransactionDisposition::Complete => {
                let Some(expected) = expected else {
                    return false;
                };
                self.kernel_eof = None;
                self.kernel_eof_seal_epoch = None;
                self.kernel_eof_committed = Some(std::sync::Arc::clone(expected));
                true
            }
            KernelEofTransactionDisposition::Preserve => true,
            KernelEofTransactionDisposition::PreserveSealed(epoch) => {
                let valid = expected.is_some_and(|expected| epoch.is_current(expected))
                    && self
                        .kernel_eof_seal_epoch
                        .as_ref()
                        .is_none_or(|current| current.same_authority(&epoch));
                if valid {
                    self.kernel_eof = None;
                    self.kernel_eof_seal_epoch = Some(epoch);
                    self.kernel_eof_committed = None;
                }
                valid
            }
            KernelEofTransactionDisposition::Revoke => {
                self.kernel_eof = None;
                self.kernel_eof_seal_epoch = None;
                self.kernel_eof_committed = None;
                false
            }
        }
    }
}
