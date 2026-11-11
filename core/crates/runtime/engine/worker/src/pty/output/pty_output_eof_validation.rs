use super::KernelPtyEofAuthority;
use super::commit::{KernelEofCommitOutcome, KernelEofValidation};
use crate::process::kernel_eof_authority::ValidatedKernelEof;
use crate::process::{
    CommittedPtyReopenSealAuthority, ProcessClosureVersion, PtyMasterAliasReleaseAuthority,
};
use crate::worker::ReaderLifecycle;
use crate::worker::reader::reader_lifecycle::KernelEofTransactionDisposition;
use anyhow::Result;
use std::fs::File;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

pub(crate) fn current_reader_validation(
    file: &File,
    authority: &KernelPtyEofAuthority,
    version: &ProcessClosureVersion,
    binding: &Arc<PtyMasterAliasReleaseAuthority>,
    lifecycle: &Arc<ReaderLifecycle>,
) -> Result<Option<KernelEofValidation>> {
    if !validate_terminal_eof(file, authority)? {
        return Ok(None);
    }
    Ok(authority
        .validate_reader(version, binding, lifecycle)
        .map(KernelEofValidation::Current))
}

pub(crate) fn current_probe_validation(
    file: &File,
    authority: &KernelPtyEofAuthority,
    version: &ProcessClosureVersion,
    binding: &Arc<PtyMasterAliasReleaseAuthority>,
    lifecycle: &Arc<ReaderLifecycle>,
) -> Result<Option<KernelEofValidation>> {
    if !validate_terminal_eof(file, authority)? {
        return Ok(None);
    }
    Ok(authority
        .validate_reader_probe(version, binding, lifecycle)
        .map(KernelEofValidation::Current))
}

pub(crate) fn sealed_validation(
    file: &File,
    authority: &mut KernelPtyEofAuthority,
    validated: ValidatedKernelEof,
    seal: &CommittedPtyReopenSealAuthority,
) -> Result<Option<KernelEofValidation>> {
    if !validate_terminal_eof(file, authority)? {
        return Ok(None);
    }
    Ok(authority
        .bind_committed_reopen_seal(validated, seal)
        .map(KernelEofValidation::ReopenSealed))
}

pub(crate) fn terminal_empty(file: &File) -> Result<bool> {
    crate::worker::pty_output_terminal::validates_kernel_pty_eof(file)
}

pub(crate) fn validate_terminal_eof(
    file: &File,
    authority: &KernelPtyEofAuthority,
) -> Result<bool> {
    match catch_unwind(AssertUnwindSafe(|| terminal_empty(file))) {
        Ok(Ok(true)) => Ok(true),
        Ok(Ok(false)) => {
            authority.invalidate_output();
            Ok(false)
        }
        Ok(Err(error)) => {
            authority.invalidate_output();
            Err(error)
        }
        Err(payload) => {
            authority.invalidate_output();
            std::panic::resume_unwind(payload)
        }
    }
}

pub(crate) fn transaction_disposition(
    outcome: KernelEofCommitOutcome,
    authority: &KernelPtyEofAuthority,
) -> KernelEofTransactionDisposition {
    match outcome {
        KernelEofCommitOutcome::Committed => KernelEofTransactionDisposition::Complete,
        KernelEofCommitOutcome::Rejected => preservation_disposition(authority),
        KernelEofCommitOutcome::OutputInvalidated => KernelEofTransactionDisposition::Revoke,
    }
}

pub(crate) fn preservation_disposition(
    authority: &KernelPtyEofAuthority,
) -> KernelEofTransactionDisposition {
    if authority.output_is_invalidated() {
        return KernelEofTransactionDisposition::Revoke;
    }
    if let Some(epoch) = authority.committed_seal_epoch() {
        return KernelEofTransactionDisposition::PreserveSealed(epoch);
    }
    KernelEofTransactionDisposition::Preserve
}
