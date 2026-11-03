use super::{OWNERSHIP_TREE_MISSING_DETAIL, PtyOwnership};
use crate::worker::pty_missing_eof::{MissingEofDeadline, MissingEofPoll, poll as poll_deadline};
use crate::worker::pty_output_progress::PtyOutputRecovery;
use anyhow::{Context, Result};
#[cfg(test)]
use std::time::Duration;
use std::time::Instant;

impl PtyOwnership {
    pub(crate) fn poll_missing_eof(
        &mut self,
        output: &mut PtyOutputRecovery,
        now: Instant,
    ) -> Result<MissingEofPoll> {
        let ownership = self.custody_mut();
        let custody = ownership
            .processes
            .as_mut()
            .context(OWNERSHIP_TREE_MISSING_DETAIL)?;
        poll_deadline(&mut ownership.missing_eof, custody, output, now)
    }

    pub(crate) fn take_missing_eof(&mut self) -> MissingEofDeadline {
        std::mem::replace(
            &mut self.custody_mut().missing_eof,
            MissingEofDeadline::production(),
        )
    }

    #[cfg(test)]
    pub(crate) fn set_missing_eof_timeout_for_test(&mut self, timeout: Duration) {
        self.custody_mut().missing_eof = MissingEofDeadline::with_timeout(timeout);
    }
}
