use super::UnreleasedBootstrapGate;
use super::pty_missing_eof::MissingEofDeadline;
use super::pty_output_progress::OutputProgressState;
use super::pty_output_progress::PtyOutputRecovery;
use super::startup_guard::{PtyChild, StartupOutputProgress};
use super::startup_untracked_proof::UntrackedStartupProducersGone;
use anyhow::{Context, Result};

#[path = "startup_untracked_reaper/driver.rs"]
mod driver;
#[path = "startup_untracked_reaper/driver_outcome.rs"]
mod driver_outcome;
#[path = "startup_untracked_reaper/identity.rs"]
mod identity;
#[path = "startup_untracked_reaper/reservation.rs"]
mod reservation;
#[path = "startup_untracked_reaper/reservation_runtime.rs"]
mod reservation_runtime;
pub(crate) use identity::StartupUntrackedIdentity;
pub(crate) use reservation::StartupUntrackedReaperReservation;

pub(crate) const READER_RECOVERY_UNAVAILABLE_CONTEXT: &str =
    "reader-backed startup output recovery is unavailable";

pub(crate) struct StartupUntrackedCleanup {
    pub(crate) child: PtyChild,
    pub(crate) reader_recovery: Option<PtyOutputRecovery>,
    pub(crate) output: StartupOutputProgress,
    pub(crate) identity: StartupUntrackedIdentity,
    pub(crate) unreleased_gate: Option<UnreleasedBootstrapGate>,
    pub(crate) producers_gone_committed: bool,
    pub(crate) wait_authority_lost: bool,
    pub(crate) missing_eof: MissingEofDeadline,
}

impl StartupUntrackedCleanup {
    #[cfg(test)]
    pub(crate) fn new(
        child: PtyChild,
        identity: StartupUntrackedIdentity,
        output: StartupOutputProgress,
        unreleased_gate: Option<UnreleasedBootstrapGate>,
        producers_gone_committed: bool,
        wait_authority_lost: bool,
    ) -> Self {
        Self::with_deadline(
            child,
            identity,
            output,
            unreleased_gate,
            producers_gone_committed,
            wait_authority_lost,
            MissingEofDeadline::production(),
        )
    }

    pub(crate) fn with_deadline(
        child: PtyChild,
        identity: StartupUntrackedIdentity,
        output: StartupOutputProgress,
        unreleased_gate: Option<UnreleasedBootstrapGate>,
        producers_gone_committed: bool,
        wait_authority_lost: bool,
        missing_eof: MissingEofDeadline,
    ) -> Self {
        let reader_recovery = match &output {
            StartupOutputProgress::Exclusive(_) => None,
            StartupOutputProgress::ReaderBacked(output) => Some(output.recovery()),
        };
        Self {
            child,
            reader_recovery,
            output,
            identity,
            unreleased_gate,
            producers_gone_committed,
            wait_authority_lost,
            missing_eof,
        }
    }

    #[cfg(test)]
    pub(crate) fn closed_gate_for_test(child: PtyChild, output: StartupOutputProgress) -> Self {
        Self::new(
            child,
            StartupUntrackedIdentity::unavailable_for_test(),
            output,
            Some(UnreleasedBootstrapGate),
            false,
            false,
        )
    }

    #[cfg(test)]
    pub(crate) fn missing_gate_for_test(child: PtyChild, output: StartupOutputProgress) -> Self {
        Self::new(
            child,
            StartupUntrackedIdentity::unavailable_for_test(),
            output,
            None,
            false,
            false,
        )
    }

    #[cfg(test)]
    pub(crate) fn lost_wait_authority_for_test(
        child: PtyChild,
        output: StartupOutputProgress,
    ) -> Self {
        Self::new(
            child,
            StartupUntrackedIdentity::unavailable_for_test(),
            output,
            Some(UnreleasedBootstrapGate),
            false,
            true,
        )
    }

    #[cfg(test)]
    pub(crate) fn set_missing_eof_timeout_for_test(&mut self, timeout: std::time::Duration) {
        self.missing_eof = MissingEofDeadline::with_timeout(timeout);
    }

    pub(crate) fn progress_output(&mut self) -> Result<OutputProgressState> {
        match &mut self.output {
            StartupOutputProgress::Exclusive(output) => output.progress(),
            StartupOutputProgress::ReaderBacked(_) => self
                .reader_recovery
                .as_mut()
                .context(READER_RECOVERY_UNAVAILABLE_CONTEXT)?
                .progress(),
        }
    }

    pub(crate) fn complete_output(
        &mut self,
        proof: UntrackedStartupProducersGone,
    ) -> Result<OutputProgressState> {
        match &mut self.output {
            StartupOutputProgress::Exclusive(output) => output.mark_untracked_producers_gone(proof),
            StartupOutputProgress::ReaderBacked(_) => self
                .reader_recovery
                .as_mut()
                .context(READER_RECOVERY_UNAVAILABLE_CONTEXT)?
                .mark_untracked_producers_gone(proof),
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_untracked_reaper_tests.rs"]
mod tests;
