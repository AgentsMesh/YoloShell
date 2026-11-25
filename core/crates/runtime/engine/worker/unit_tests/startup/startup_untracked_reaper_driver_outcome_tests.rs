use super::*;
use crate::worker::pty_missing_eof::MissingEofDeadline;
use std::time::{Duration, Instant};

const OUTCOME_ERROR_DETAIL: &str = "injected driver outcome error";
const TEST_EXIT_CODE: u32 = 0;

#[derive(Clone, Copy)]
enum DriverOutcomeAction {
    Complete,
    Abandoned,
    Pending,
    Quiescent,
    Matches,
    Gone,
    Exited,
    Running,
    Error,
    AuthorityLoss,
    Panic,
}

#[test]
fn output_and_missing_eof_outcomes_share_one_action_matrix() {
    for (action, expected) in [
        (DriverOutcomeAction::Complete, true),
        (DriverOutcomeAction::Abandoned, true),
        (DriverOutcomeAction::Pending, false),
        (DriverOutcomeAction::Quiescent, false),
        (DriverOutcomeAction::Error, false),
        (DriverOutcomeAction::Panic, false),
    ] {
        assert_eq!(output_complete(action.output()), expected);
    }
    for (action, expected) in [
        (DriverOutcomeAction::Abandoned, true),
        (DriverOutcomeAction::Pending, false),
        (DriverOutcomeAction::Error, false),
        (DriverOutcomeAction::Panic, false),
    ] {
        assert_eq!(missing_eof_settled(action.missing_eof()), expected);
    }
}

#[test]
fn identity_and_child_outcomes_share_one_action_matrix() {
    for (action, expected) in [
        (DriverOutcomeAction::Matches, false),
        (DriverOutcomeAction::Gone, true),
        (DriverOutcomeAction::Error, false),
        (DriverOutcomeAction::Panic, false),
    ] {
        assert_eq!(identity_gone(action.identity()), expected);
    }
    for (action, expected, authority_lost) in [
        (DriverOutcomeAction::Exited, true, false),
        (DriverOutcomeAction::Running, false, false),
        (DriverOutcomeAction::Error, false, false),
        (DriverOutcomeAction::AuthorityLoss, false, true),
        (DriverOutcomeAction::Panic, false, false),
    ] {
        let mut lost = false;
        assert_eq!(child_exited(action.child(), &mut lost), expected);
        assert_eq!(lost, authority_lost);
    }
}

impl DriverOutcomeAction {
    fn output(self) -> Option<Result<OutputProgressState>> {
        match self {
            Self::Complete => Some(Ok(OutputProgressState::Complete)),
            Self::Abandoned => Some(Ok(OutputProgressState::Abandoned)),
            Self::Pending => Some(Ok(OutputProgressState::Pending)),
            Self::Quiescent => Some(Ok(OutputProgressState::Quiescent)),
            Self::Error => Some(Err(anyhow::anyhow!(OUTCOME_ERROR_DETAIL))),
            Self::Panic => None,
            _ => unreachable!(),
        }
    }

    fn missing_eof(self) -> Option<Result<OutputAbandonment>> {
        match self {
            Self::Abandoned => Some(Ok(abandoned_output())),
            Self::Pending => Some(Ok(OutputAbandonment::Retry)),
            Self::Error => Some(Err(anyhow::anyhow!(OUTCOME_ERROR_DETAIL))),
            Self::Panic => None,
            _ => unreachable!(),
        }
    }

    fn identity(self) -> Option<Result<bool>> {
        match self {
            Self::Matches => Some(Ok(true)),
            Self::Gone => Some(Ok(false)),
            Self::Error => Some(Err(anyhow::anyhow!(OUTCOME_ERROR_DETAIL))),
            Self::Panic => None,
            _ => unreachable!(),
        }
    }

    fn child(self) -> Option<Result<Option<portable_pty::ExitStatus>>> {
        match self {
            Self::Exited => Some(Ok(Some(portable_pty::ExitStatus::with_exit_code(
                TEST_EXIT_CODE,
            )))),
            Self::Running => Some(Ok(None)),
            Self::Error => Some(Err(anyhow::anyhow!(OUTCOME_ERROR_DETAIL))),
            Self::AuthorityLoss => Some(Err(anyhow::Error::new(
                std::io::Error::from_raw_os_error(libc::ECHILD),
            ))),
            Self::Panic => None,
            _ => unreachable!(),
        }
    }
}

fn abandoned_output() -> OutputAbandonment {
    let mut deadline = MissingEofDeadline::with_timeout(Duration::ZERO);
    let now = Instant::now();
    let expired = deadline
        .observe(OutputProgressState::Quiescent, now)
        .unwrap()
        .unwrap();
    OutputAbandonment::Abandoned(expired.abandon())
}
