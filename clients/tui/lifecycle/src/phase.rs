use crate::labels::*;
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalSummaryFields};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalPhase {
    Starting,
    Running,
    Finalizing,
    OutputBlocked,
    Ended,
    Lost,
    Archived,
    Unreachable,
    Unavailable,
}

pub fn terminal_phase(
    terminal: &TerminalSummaryFields,
    ended: Option<RuntimeEpoch>,
) -> TerminalPhase {
    if terminal.archived {
        return TerminalPhase::Archived;
    }
    let Some(runtime) = terminal.runtime else {
        return TerminalPhase::Unavailable;
    };
    if ended == Some(runtime.target.runtime_epoch) {
        return TerminalPhase::Ended;
    }
    match runtime.status {
        RuntimeStatus::Starting => TerminalPhase::Starting,
        RuntimeStatus::Running => TerminalPhase::Running,
        RuntimeStatus::Finalizing => TerminalPhase::Finalizing,
        RuntimeStatus::OutputBlocked => TerminalPhase::OutputBlocked,
        RuntimeStatus::Exited => TerminalPhase::Ended,
        RuntimeStatus::Lost => TerminalPhase::Lost,
        RuntimeStatus::Unreachable => TerminalPhase::Unreachable,
    }
}

impl TerminalPhase {
    pub fn allows_input(self) -> bool {
        self == Self::Running
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Starting => STARTING_LABEL,
            Self::Running => RUNNING_LABEL,
            Self::Finalizing => FINALIZING_LABEL,
            Self::OutputBlocked => OUTPUT_BLOCKED_LABEL,
            Self::Ended => ENDED_LABEL,
            Self::Lost => LOST_LABEL,
            Self::Archived => ARCHIVED_LABEL,
            Self::Unreachable => UNREACHABLE_LABEL,
            Self::Unavailable => UNAVAILABLE_LABEL,
        }
    }

    pub fn input_blocked_hint(self) -> Option<&'static str> {
        Some(match self {
            Self::Running => return None,
            Self::Starting => STARTING_HINT,
            Self::Finalizing => FINALIZING_HINT,
            Self::OutputBlocked => OUTPUT_BLOCKED_HINT,
            Self::Ended | Self::Archived => ENDED_HINT,
            Self::Lost => LOST_HINT,
            Self::Unreachable => UNREACHABLE_HINT,
            Self::Unavailable => UNAVAILABLE_HINT,
        })
    }
}
