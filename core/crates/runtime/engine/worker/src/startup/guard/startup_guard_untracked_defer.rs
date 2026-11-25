use super::super::pty_missing_eof::MissingEofDeadline;
use super::super::startup_untracked_reaper::StartupUntrackedCleanup;
use super::{
    STARTUP_CHILD_UNAVAILABLE_CONTEXT, StartupChildGuard, StartupOutputProgress,
    untracked::{UNTRACKED_CLEANUP_DEFERRED_DETAIL, UNTRACKED_REAPER_UNAVAILABLE_CONTEXT},
};
use anyhow::{Context, Result, anyhow};

impl StartupChildGuard {
    pub(crate) fn defer_untracked(
        &mut self,
        failure: Option<anyhow::Error>,
        output: StartupOutputProgress,
        wait_authority_lost: bool,
        missing_eof: MissingEofDeadline,
    ) -> Result<()> {
        self.handoff_untracked(output, wait_authority_lost, missing_eof)?;
        Err(match failure {
            Some(error) => error.context(UNTRACKED_CLEANUP_DEFERRED_DETAIL),
            None => anyhow!(UNTRACKED_CLEANUP_DEFERRED_DETAIL),
        })
    }

    pub(crate) fn handoff_untracked(
        &mut self,
        output: StartupOutputProgress,
        wait_authority_lost: bool,
        missing_eof: MissingEofDeadline,
    ) -> Result<()> {
        self.untracked_reaper
            .as_ref()
            .context(UNTRACKED_REAPER_UNAVAILABLE_CONTEXT)?;
        self.child
            .as_ref()
            .context(STARTUP_CHILD_UNAVAILABLE_CONTEXT)?;
        let identity = self.take_untracked_identity();
        let reservation = self
            .untracked_reaper
            .take()
            .context(UNTRACKED_REAPER_UNAVAILABLE_CONTEXT)?;
        let child = self
            .child
            .take()
            .context(STARTUP_CHILD_UNAVAILABLE_CONTEXT)?;
        let cleanup = StartupUntrackedCleanup::with_deadline(
            child,
            identity,
            output,
            self.unreleased_gate.take(),
            self.untracked_gone_committed,
            wait_authority_lost,
            missing_eof,
        );
        reservation.handoff(cleanup);
        self.clear();
        Ok(())
    }
}
