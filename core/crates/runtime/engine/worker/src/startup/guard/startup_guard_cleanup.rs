use super::super::constants::STARTUP_CHILD_EXIT_POLL_INTERVAL;
use super::super::pty_fault_reaper::settle_inline;
use super::super::pty_output_progress::ReaderBackedPtyOutput;
use super::{StartupChildGuard, StartupOutputProgress};
use anyhow::{Context, Result, ensure};

pub(crate) const STARTUP_OUTPUT_STILL_OWNED_DETAIL: &str =
    "startup cleanup cannot substitute released output while output authority is still owned";

impl StartupChildGuard {
    #[cfg(test)]
    pub(crate) fn terminate(&mut self) -> Result<()> {
        if self.output_progress.is_some() {
            return self.terminate_with_owned_output();
        }
        self.terminate_with(|| Ok(()))
    }

    pub(crate) fn terminate_with<Progress>(&mut self, progress: Progress) -> Result<()>
    where
        Progress: FnMut() -> Result<()> + Send,
    {
        ensure!(
            self.output_progress.is_none(),
            STARTUP_OUTPUT_STILL_OWNED_DETAIL
        );
        let failure = self.stop_monitor().err();
        if self.tree.is_some() {
            return self.terminate_tracked(failure, progress);
        }
        self.terminate_untracked_with_output(
            failure,
            StartupOutputProgress::ReaderBacked(ReaderBackedPtyOutput::after_master_release()),
        )
    }

    pub(crate) fn terminate_tracked<Progress>(
        &mut self,
        mut failure: Option<anyhow::Error>,
        progress: Progress,
    ) -> Result<()>
    where
        Progress: FnMut() -> Result<()> + Send,
    {
        let state_observer = self
            .fault_reaper
            .as_ref()
            .context(super::STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT)?
            .state_observer();
        loop {
            match self.take_stopped_authority() {
                Ok((authority, tree)) => {
                    let mut progress = progress;
                    let progress_result = progress();
                    if let Err(error) = progress_result {
                        remember(&mut failure, error);
                    }
                    let outcome = settle_inline(
                        authority,
                        tree,
                        ReaderBackedPtyOutput::after_master_release(),
                        failure,
                        state_observer,
                    );
                    self.clear();
                    return outcome.fault.map_or(Ok(()), Err);
                }
                Err(error) => remember(&mut failure, error),
            }
            std::thread::sleep(STARTUP_CHILD_EXIT_POLL_INTERVAL);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.child = None;
        self.authority = None;
        self.tree = None;
        self.output_progress = None;
        self.untracked_reaper = None;
        self.fault_reaper = None;
        self.unreleased_gate = None;
        self.untracked_gone_committed = false;
        self.identity = None;
        self.live_identity = None;
        self.process_id = None;
    }
}

pub(crate) fn remember(failure: &mut Option<anyhow::Error>, error: anyhow::Error) {
    if failure.is_none() {
        *failure = Some(error);
    } else {
        super::super::terminal_reporting::discard_error(error);
    }
}
