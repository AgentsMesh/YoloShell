use super::*;

const RETAINED_UNCOMMITTED_PROOF_DETAIL: &str = "retained append cannot be proven uncommitted";

#[derive(Debug)]
pub enum RetainedAppendOutcome {
    Confirmed(OutputChunk),
    DefinitelyUncommitted(JournalError),
    CommitUncertain(JournalError),
}

#[derive(Debug)]
pub enum RetainedAppendBatchOutcome {
    Confirmed(Vec<OutputChunk>),
    DefinitelyUncommitted(JournalError),
    CommitUncertain(JournalError),
}

pub(super) enum RetainedAppendFailure {
    BeforePublication(JournalError),
    CommitUncertain(JournalError),
}

impl Journal {
    pub fn reconcile_append_retained_classified(
        &mut self,
        attempt: AppendAttempt,
        payload: &[u8],
    ) -> RetainedAppendOutcome {
        match self.reconcile_append_retained_attempt(attempt, payload) {
            Ok(chunk) => RetainedAppendOutcome::Confirmed(chunk),
            Err(RetainedAppendFailure::CommitUncertain(error)) => {
                RetainedAppendOutcome::CommitUncertain(error)
            }
            Err(RetainedAppendFailure::BeforePublication(error)) => {
                if self
                    .prove_retained_append_uncommitted(attempt, payload)
                    .is_ok()
                {
                    RetainedAppendOutcome::DefinitelyUncommitted(error)
                } else {
                    RetainedAppendOutcome::CommitUncertain(error)
                }
            }
        }
    }

    pub fn reconcile_append_retained_batch_classified(
        &mut self,
        attempt: AppendAttempt,
        payloads: &[&[u8]],
    ) -> RetainedAppendBatchOutcome {
        match self.reconcile_append_retained_batch_attempt(attempt, payloads) {
            Ok(chunks) => RetainedAppendBatchOutcome::Confirmed(chunks),
            Err(RetainedAppendBatchFailure::CommitUncertain(error)) => {
                RetainedAppendBatchOutcome::CommitUncertain(error)
            }
            Err(RetainedAppendBatchFailure::BeforePublication(error)) => {
                if self
                    .prove_retained_batch_uncommitted(attempt, payloads)
                    .is_ok()
                {
                    RetainedAppendBatchOutcome::DefinitelyUncommitted(error)
                } else {
                    RetainedAppendBatchOutcome::CommitUncertain(error)
                }
            }
        }
    }

    fn prove_retained_append_uncommitted(
        &mut self,
        attempt: AppendAttempt,
        payload: &[u8],
    ) -> Result<(), JournalError> {
        let expected = self.expected_retained_append(attempt, payload)?;
        let _lock = self.entry.lock_retained_parent()?;
        self.commit_cursor
            .verify_retained_evidence(attempt.cursor)?;
        let before = self.verify_retained_journal(self.max_disk_bytes)?;
        if self.commit_cursor.record.committed_bytes != expected.base
            || before.length() < expected.base
        {
            return Err(anyhow::anyhow!(RETAINED_UNCOMMITTED_PROOF_DETAIL).into());
        }
        let prefix = scan::scan_committed(&mut self.file, expected.base, self.runtime_epoch)?;
        if prefix.last_sequence != attempt.previous_sequence {
            return Err(JournalError::Corrupt {
                sequence: prefix.last_sequence,
            });
        }
        self.entry.sync_retained_parent()?;
        self.commit_cursor
            .verify_retained_evidence(attempt.cursor)?;
        let after = self.verify_retained_journal(self.max_disk_bytes)?;
        if !before.unchanged(after) {
            return Err(anyhow::anyhow!(RETAINED_UNCOMMITTED_PROOF_DETAIL).into());
        }
        Ok(())
    }
}

pub(super) enum RetainedAppendBatchFailure {
    BeforePublication(JournalError),
    CommitUncertain(JournalError),
}

impl RetainedAppendBatchOutcome {
    pub(super) fn into_result(self) -> Result<Vec<OutputChunk>, JournalError> {
        match self {
            Self::Confirmed(chunks) => Ok(chunks),
            Self::DefinitelyUncommitted(error) | Self::CommitUncertain(error) => Err(error),
        }
    }
}

impl RetainedAppendOutcome {
    pub(super) fn into_result(self) -> Result<OutputChunk, JournalError> {
        match self {
            Self::Confirmed(chunk) => Ok(chunk),
            Self::DefinitelyUncommitted(error) | Self::CommitUncertain(error) => Err(error),
        }
    }
}
