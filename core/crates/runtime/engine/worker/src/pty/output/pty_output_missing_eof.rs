use super::{ExclusivePtyOutput, OutputProgressState, PtyOutputRecovery};
use crate::worker::pty_missing_eof::{MissingEofExpired, MissingEofOutput, OutputAbandonment};
use anyhow::Result;

impl MissingEofOutput for PtyOutputRecovery {
    fn missing_eof_progress(&mut self) -> Result<OutputProgressState> {
        self.progress()
    }

    fn settle_missing_eof(&mut self, proof: MissingEofExpired) -> Result<OutputAbandonment> {
        Ok(self
            .reader_lifecycle
            .as_ref()
            .map_or(OutputAbandonment::Retry, |lifecycle| {
                lifecycle.settle_missing_eof(proof)
            }))
    }
}

impl MissingEofOutput for ExclusivePtyOutput {
    fn missing_eof_progress(&mut self) -> Result<OutputProgressState> {
        self.progress()
    }

    fn settle_missing_eof(&mut self, proof: MissingEofExpired) -> Result<OutputAbandonment> {
        if self.kernel_eof.is_some()
            || self.kernel_eof_seal_epoch.is_some()
            || self.kernel_eof_committed.is_some()
        {
            return Ok(OutputAbandonment::Retry);
        }
        Ok(match self.progress()? {
            OutputProgressState::Quiescent => {
                self.output_abandoned = true;
                self.kernel_eof = None;
                self.kernel_eof_seal_epoch = None;
                self.kernel_eof_committed = None;
                OutputAbandonment::Abandoned(proof.abandon())
            }
            OutputProgressState::Pending
            | OutputProgressState::Complete
            | OutputProgressState::Abandoned => OutputAbandonment::Retry,
        })
    }
}
