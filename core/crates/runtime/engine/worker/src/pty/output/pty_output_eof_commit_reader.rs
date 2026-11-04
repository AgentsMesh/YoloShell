use super::operation::CapturedKernelEofOperation;
use super::{KernelEofCommitOutcome, KernelEofValidationRequest, Validator};
use crate::process::ProcessClosureVersion;
use crate::worker::pty_output_progress::{
    KernelPtyEofAuthority, OutputDescriptor, PtyOutputRecovery,
};
use anyhow::Result;

use super::super::validation::{
    current_probe_validation, current_reader_validation, sealed_validation, terminal_empty,
};

impl PtyOutputRecovery {
    pub(crate) fn commit_if_kernel_eof_current(
        &mut self,
        authority: &mut KernelPtyEofAuthority,
        version: &ProcessClosureVersion,
        operation: impl FnOnce(&mut Validator<'_>) -> Result<KernelEofCommitOutcome>,
    ) -> Result<bool> {
        let (OutputDescriptor::Native(file), Some(lifecycle), Some(binding)) = (
            &*self.descriptor.lock(),
            self.reader_lifecycle.as_ref(),
            self.master_alias_authority.as_ref(),
        ) else {
            return Ok(false);
        };
        let reader = authority
            .reader_identity(version, binding, lifecycle)
            .cloned();
        let probe = authority
            .reader_probe_identity(version, binding, lifecycle)
            .cloned();
        let mut completion = None;
        let accepted = match (reader, probe) {
            (Some(identity), _) => lifecycle.with_validated_current_kernel_eof(
                &identity,
                || terminal_empty(file),
                || {
                    let mut validate = |request| match request {
                        KernelEofValidationRequest::Current => {
                            current_reader_validation(file, authority, version, binding, lifecycle)
                        }
                        KernelEofValidationRequest::ReopenSealed { validated, seal } => {
                            sealed_validation(file, authority, validated, &seal)
                        }
                    };
                    let captured = CapturedKernelEofOperation::run(|| operation(&mut validate));
                    drop(validate);
                    let disposition = captured.disposition(authority);
                    completion = Some(captured);
                    Ok(disposition)
                },
            )?,
            (None, Some(identity)) => lifecycle.with_validated_current_kernel_eof_probe(
                &identity,
                || crate::worker::pty_output_terminal::validates_kernel_pty_eof(file),
                || {
                    let mut validate = |request| match request {
                        KernelEofValidationRequest::Current => {
                            current_probe_validation(file, authority, version, binding, lifecycle)
                        }
                        KernelEofValidationRequest::ReopenSealed { validated, seal } => {
                            sealed_validation(file, authority, validated, &seal)
                        }
                    };
                    let captured = CapturedKernelEofOperation::run(|| operation(&mut validate));
                    drop(validate);
                    let disposition = captured.disposition(authority);
                    completion = Some(captured);
                    Ok(disposition)
                },
            )?,
            (None, None) => return Ok(false),
        };
        let Some(completion) = completion else {
            return Ok(false);
        };
        let outcome = completion.finish()?;
        let committed = accepted && outcome == KernelEofCommitOutcome::Committed;
        if committed || outcome == KernelEofCommitOutcome::OutputInvalidated {
            authority.retire_validation();
        }
        Ok(committed)
    }
}
