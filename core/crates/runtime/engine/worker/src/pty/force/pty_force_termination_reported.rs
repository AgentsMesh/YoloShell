use crate::process::{ReportedProducersGone, ReportedProducersGoneRetirementReady};
use crate::worker::pty_output_progress::OutputProgressState;
use anyhow::Result;

pub(crate) enum ReportedProgressReady {
    Waiting,
    Retirement(ReportedProducersGoneRetirementReady),
}

impl ReportedProgressReady {
    pub(crate) fn prepare(
        completion: &mut ReportedProducersGone,
        progress: OutputProgressState,
    ) -> Result<Self> {
        match progress {
            OutputProgressState::Complete | OutputProgressState::Abandoned => completion
                .prepare_retirement()
                .map(ReportedProgressReady::Retirement),
            OutputProgressState::Pending | OutputProgressState::Quiescent => Ok(Self::Waiting),
        }
    }
}
