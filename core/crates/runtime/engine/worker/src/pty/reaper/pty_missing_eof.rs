use super::pty_output_progress::OutputProgressState;
use super::termination_timing::PTY_MISSING_EOF_TIMEOUT;
use anyhow::{Context, Result};
use std::time::{Duration, Instant};

pub(crate) const DEADLINE_OVERFLOW_CONTEXT: &str = "compute PTY missing-EOF deadline";

pub(crate) struct MissingEofDeadline {
    pub(crate) expires_at: Option<Instant>,
    pub(crate) timeout: Duration,
}

pub(crate) struct MissingEofExpired {
    pub(crate) _private: (),
}

pub(crate) struct AbandonedPtyOutput {
    pub(crate) _private: (),
}

pub(crate) enum OutputAbandonment {
    Retry,
    Abandoned(AbandonedPtyOutput),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MissingEofPoll {
    Pending,
    Lost,
}

pub(crate) trait MissingEofCustody {
    fn producers_are_reported_gone(&self) -> bool;
    fn retire_abandoned_output(&mut self, proof: AbandonedPtyOutput) -> Result<()>;
}

pub(crate) trait MissingEofOutput {
    fn missing_eof_progress(&mut self) -> Result<OutputProgressState>;
    fn settle_missing_eof(&mut self, proof: MissingEofExpired) -> Result<OutputAbandonment>;
}

impl MissingEofDeadline {
    pub(crate) fn production() -> Self {
        Self::with_timeout(PTY_MISSING_EOF_TIMEOUT)
    }

    pub(crate) fn with_timeout(timeout: Duration) -> Self {
        Self {
            expires_at: None,
            timeout,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.expires_at = None;
    }

    pub(crate) fn observe(
        &mut self,
        progress: OutputProgressState,
        now: Instant,
    ) -> Result<Option<MissingEofExpired>> {
        if progress != OutputProgressState::Quiescent {
            self.reset();
            return Ok(None);
        }
        let expires_at = match self.expires_at {
            Some(expires_at) => expires_at,
            None => {
                let expires_at = now
                    .checked_add(self.timeout)
                    .context(DEADLINE_OVERFLOW_CONTEXT)?;
                self.expires_at = Some(expires_at);
                expires_at
            }
        };
        Ok((now >= expires_at).then_some(MissingEofExpired { _private: () }))
    }
}

impl MissingEofExpired {
    pub(crate) fn abandon(self) -> AbandonedPtyOutput {
        AbandonedPtyOutput { _private: () }
    }
}

pub(crate) fn poll<Custody, Output>(
    deadline: &mut MissingEofDeadline,
    custody: &mut Custody,
    output: &mut Output,
    now: Instant,
) -> Result<MissingEofPoll>
where
    Custody: MissingEofCustody,
    Output: MissingEofOutput,
{
    let producers_gone = custody.producers_are_reported_gone();
    match poll_output(deadline, producers_gone, output, now)? {
        OutputAbandonment::Retry => Ok(MissingEofPoll::Pending),
        OutputAbandonment::Abandoned(proof) => {
            custody.retire_abandoned_output(proof)?;
            Ok(MissingEofPoll::Lost)
        }
    }
}

pub(crate) fn poll_output<Output>(
    deadline: &mut MissingEofDeadline,
    producers_are_reported_gone: bool,
    output: &mut Output,
    now: Instant,
) -> Result<OutputAbandonment>
where
    Output: MissingEofOutput,
{
    if !producers_are_reported_gone {
        deadline.reset();
        return Ok(OutputAbandonment::Retry);
    }
    let progress = output.missing_eof_progress()?;
    let Some(expired) = deadline.observe(progress, now)? else {
        return Ok(OutputAbandonment::Retry);
    };
    let settlement = output.settle_missing_eof(expired)?;
    if matches!(settlement, OutputAbandonment::Retry) {
        deadline.reset();
    }
    Ok(settlement)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/missing/pty_missing_eof_reader_tests.rs"]
mod reader_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/missing/pty_missing_eof_tests.rs"]
mod tests;
