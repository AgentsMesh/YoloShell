use super::KernelEofCommitOutcome;
use crate::worker::pty_output_progress::KernelPtyEofAuthority;
use crate::worker::reader::reader_lifecycle::KernelEofTransactionDisposition;
use anyhow::{Error, Result};
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(crate) enum CapturedKernelEofOperation {
    Outcome(KernelEofCommitOutcome),
    Error(Error),
    Panic(Box<dyn Any + Send>),
}

impl CapturedKernelEofOperation {
    pub(crate) fn run(operation: impl FnOnce() -> Result<KernelEofCommitOutcome>) -> Self {
        match catch_unwind(AssertUnwindSafe(operation)) {
            Ok(Ok(outcome)) => Self::Outcome(outcome),
            Ok(Err(error)) => Self::Error(error),
            Err(payload) => Self::Panic(payload),
        }
    }

    pub(crate) fn disposition(
        &self,
        authority: &KernelPtyEofAuthority,
    ) -> KernelEofTransactionDisposition {
        match self {
            Self::Outcome(outcome) => {
                super::super::validation::transaction_disposition(*outcome, authority)
            }
            Self::Error(error)
                if crate::process::FrozenProcessTree::kernel_eof_error_invalidates_output(
                    error,
                ) =>
            {
                KernelEofTransactionDisposition::Revoke
            }
            Self::Error(_) if authority.committed_seal_epoch().is_some() => {
                KernelEofTransactionDisposition::Revoke
            }
            Self::Panic(_) => KernelEofTransactionDisposition::Revoke,
            Self::Error(_) => super::super::validation::preservation_disposition(authority),
        }
    }

    pub(crate) fn finish(self) -> Result<KernelEofCommitOutcome> {
        match self {
            Self::Outcome(outcome) => Ok(outcome),
            Self::Error(error) => Err(error),
            Self::Panic(payload) => std::panic::resume_unwind(payload),
        }
    }
}
