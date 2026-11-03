use crate::process::CommittedPtyReopenSealAuthority;
use crate::process::kernel_eof_authority::{ValidatedKernelEof, ValidatedSealedKernelEof};
use anyhow::Result;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum KernelEofCommitOutcome {
    Committed,
    Rejected,
    OutputInvalidated,
}

pub(crate) enum KernelEofValidationRequest {
    Current,
    ReopenSealed {
        validated: ValidatedKernelEof,
        seal: CommittedPtyReopenSealAuthority,
    },
}

pub(crate) enum KernelEofValidation {
    Current(ValidatedKernelEof),
    ReopenSealed(ValidatedSealedKernelEof),
}

pub(crate) type Validator<'a> =
    dyn FnMut(KernelEofValidationRequest) -> Result<Option<KernelEofValidation>> + 'a;

#[path = "pty_output_eof_commit_exclusive.rs"]
mod exclusive;
#[path = "pty_output_eof_commit_operation.rs"]
mod operation;
#[path = "pty_output_eof_commit_reader.rs"]
mod reader;
