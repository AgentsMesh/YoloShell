use super::super::pty_missing_eof::{MissingEofDeadline, OutputAbandonment, poll_output};
use super::super::pty_output_progress::{OutputProgressState, PtyOutputRecovery};
use super::super::startup_untracked_proof::UntrackedStartupProducersGone;
use super::StartupOutputProgress;
use anyhow::{Context, Result, anyhow};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

pub(crate) const UNTRACKED_PROGRESS_PANIC_DETAIL: &str =
    "untracked startup output progress panicked";
pub(crate) const UNTRACKED_READER_RECOVERY_CONTEXT: &str =
    "untracked reader-backed output recovery is unavailable";
pub(crate) const UNTRACKED_OUTPUT_ABANDONED_DETAIL: &str =
    "untracked startup PTY output was abandoned after missing EOF";

pub(crate) fn progress_output(
    output: &mut StartupOutputProgress,
    reader: &mut Option<PtyOutputRecovery>,
) -> Result<OutputProgressState> {
    match output {
        StartupOutputProgress::Exclusive(output) => output.progress(),
        StartupOutputProgress::ReaderBacked(_) => reader
            .as_mut()
            .context(UNTRACKED_READER_RECOVERY_CONTEXT)?
            .progress(),
    }
}

pub(crate) fn complete_output(
    output: &mut StartupOutputProgress,
    reader: &mut Option<PtyOutputRecovery>,
    proof: UntrackedStartupProducersGone,
) -> Result<OutputProgressState> {
    match output {
        StartupOutputProgress::Exclusive(output) => output.mark_untracked_producers_gone(proof),
        StartupOutputProgress::ReaderBacked(_) => reader
            .as_mut()
            .context(UNTRACKED_READER_RECOVERY_CONTEXT)?
            .mark_untracked_producers_gone(proof),
    }
}

pub(crate) fn contain_progress(
    failure: &mut Option<anyhow::Error>,
    progress: impl FnOnce() -> Result<OutputProgressState>,
) -> bool {
    match catch_unwind(AssertUnwindSafe(progress)) {
        Ok(Ok(OutputProgressState::Complete | OutputProgressState::Abandoned)) => true,
        Ok(Ok(OutputProgressState::Pending | OutputProgressState::Quiescent)) => false,
        Ok(Err(error)) => {
            super::cleanup::remember(failure, error);
            false
        }
        Err(payload) => {
            let detail = super::super::supervision::panic_detail(payload);
            super::cleanup::remember(
                failure,
                anyhow!("{UNTRACKED_PROGRESS_PANIC_DETAIL}: {detail}"),
            );
            false
        }
    }
}

pub(crate) fn contain_missing_eof(
    failure: &mut Option<anyhow::Error>,
    deadline: &mut MissingEofDeadline,
    output: &mut StartupOutputProgress,
    reader: &mut Option<PtyOutputRecovery>,
) -> bool {
    let settled = catch_unwind(AssertUnwindSafe(|| match output {
        StartupOutputProgress::Exclusive(output) => {
            poll_output(deadline, true, output, Instant::now())
        }
        StartupOutputProgress::ReaderBacked(_) => poll_output(
            deadline,
            true,
            reader.as_mut().context(UNTRACKED_READER_RECOVERY_CONTEXT)?,
            Instant::now(),
        ),
    }));
    match settled {
        Ok(Ok(OutputAbandonment::Retry)) => false,
        Ok(Ok(OutputAbandonment::Abandoned(_proof))) => {
            super::cleanup::remember(failure, anyhow!(UNTRACKED_OUTPUT_ABANDONED_DETAIL));
            true
        }
        Ok(Err(error)) => {
            super::cleanup::remember(failure, error);
            false
        }
        Err(payload) => {
            let detail = super::super::supervision::panic_detail(payload);
            super::cleanup::remember(
                failure,
                anyhow!("{UNTRACKED_PROGRESS_PANIC_DETAIL}: {detail}"),
            );
            false
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_guard_untracked_progress_tests.rs"]
mod tests;
