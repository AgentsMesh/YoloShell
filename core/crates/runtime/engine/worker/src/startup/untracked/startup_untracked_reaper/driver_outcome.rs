use super::super::pty_missing_eof::OutputAbandonment;
use super::super::pty_output_progress::OutputProgressState;
use crate::process::is_wait_authority_loss;
use anyhow::Result;

pub(crate) const CALLBACK_ERROR_DETAIL: &str = "untracked startup reaper callback failed";
pub(crate) const OUTPUT_ABANDONED_DETAIL: &str =
    "untracked startup reaper abandoned PTY output after missing EOF";

pub(crate) fn identity_gone(result: Option<Result<bool>>) -> bool {
    match result {
        Some(Ok(matches)) => !matches,
        Some(Err(error)) => {
            warn(error);
            false
        }
        None => false,
    }
}

pub(crate) fn output_complete(result: Option<Result<OutputProgressState>>) -> bool {
    match result {
        Some(Ok(OutputProgressState::Complete | OutputProgressState::Abandoned)) => true,
        Some(Ok(OutputProgressState::Pending | OutputProgressState::Quiescent)) => false,
        Some(Err(error)) => {
            warn(error);
            false
        }
        None => false,
    }
}

pub(crate) fn missing_eof_settled(result: Option<Result<OutputAbandonment>>) -> bool {
    match result {
        Some(Ok(OutputAbandonment::Abandoned(_proof))) => {
            super::super::terminal_reporting::warn_message(OUTPUT_ABANDONED_DETAIL);
            true
        }
        Some(Ok(OutputAbandonment::Retry)) => false,
        Some(Err(error)) => {
            warn(error);
            false
        }
        None => false,
    }
}

pub(crate) fn child_exited(
    result: Option<Result<Option<portable_pty::ExitStatus>>>,
    wait_authority_lost: &mut bool,
) -> bool {
    match result {
        Some(Ok(Some(_))) => true,
        Some(Ok(None)) | None => false,
        Some(Err(error)) => {
            *wait_authority_lost =
                super::super::supervision::contain(|| is_wait_authority_loss(&error))
                    .unwrap_or(false);
            warn(error);
            false
        }
    }
}

pub(crate) fn warn(error: anyhow::Error) {
    super::super::terminal_reporting::warn_error(error, CALLBACK_ERROR_DETAIL);
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../../unit_tests/startup/startup_untracked_reaper_driver_containment_tests.rs"]
mod containment_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../../unit_tests/startup/startup_untracked_reaper_driver_outcome_tests.rs"]
mod tests;
