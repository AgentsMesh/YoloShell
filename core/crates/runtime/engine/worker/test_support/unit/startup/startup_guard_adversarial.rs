use super::super::pty_missing_eof::MissingEofDeadline;
use super::{
    STARTUP_AUTHORITY_UNAVAILABLE_CONTEXT, STARTUP_OUTPUT_PROGRESS_UNAVAILABLE_CONTEXT,
    StartupChildGuard, untracked::UNTRACKED_REAPER_UNAVAILABLE_CONTEXT,
};
use anyhow::{Context, Result};

impl StartupChildGuard {
    pub(in crate::worker) fn handoff_to_untracked_reaper(&mut self) -> Result<()> {
        self.authority
            .as_ref()
            .context(STARTUP_AUTHORITY_UNAVAILABLE_CONTEXT)?;
        self.untracked_reaper
            .as_ref()
            .context(UNTRACKED_REAPER_UNAVAILABLE_CONTEXT)?;
        self.output_progress
            .as_ref()
            .context(STARTUP_OUTPUT_PROGRESS_UNAVAILABLE_CONTEXT)?;
        let monitor = self.stop_monitor();
        let authority = self
            .authority
            .take()
            .context(STARTUP_AUTHORITY_UNAVAILABLE_CONTEXT)?;
        self.child = Some(authority.into_untracked_child());
        self.tree = None;
        let output = self
            .output_progress
            .take()
            .context(STARTUP_OUTPUT_PROGRESS_UNAVAILABLE_CONTEXT)?;
        self.handoff_untracked(output, false, MissingEofDeadline::production())?;
        monitor
    }
}
