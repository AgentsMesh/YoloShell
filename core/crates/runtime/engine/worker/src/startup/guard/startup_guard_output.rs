use super::super::constants::STARTUP_CHILD_EXIT_POLL_INTERVAL;
use super::super::pty_fault_reaper::{settle_inline, settle_inline_after_master_release};
use super::super::pty_output_progress::OutputProgressState;
use super::super::pty_output_progress::{ExclusivePtyOutput, ReaderBackedPtyOutput};
use super::super::pty_reader_activation::{PendingPtyReader, PreparedPtyReader};
use super::{
    STARTUP_OUTPUT_PROGRESS_UNAVAILABLE_CONTEXT, STARTUP_READER_ALREADY_INSTALLED_CONTEXT,
    StartupChildGuard, StartupOutputProgress,
};
use anyhow::{Context, Result, bail};

pub(crate) const PREPARED_READER_INVARIANT: &str =
    "startup guard retains exclusive output until prepared reader activation";

impl StartupChildGuard {
    #[cfg(test)]
    pub(crate) fn install_reader_backed_output_for_test(&mut self) {
        let output = match self.output_progress.take() {
            Some(StartupOutputProgress::Exclusive(output)) => output.into_reader_backed_for_test(),
            Some(StartupOutputProgress::ReaderBacked(output)) => output,
            None => ReaderBackedPtyOutput::for_test(),
        };
        self.output_progress = Some(StartupOutputProgress::ReaderBacked(output));
    }

    #[cfg(test)]
    pub(crate) fn activate_reader(&mut self, reader: PendingPtyReader) -> Result<()> {
        let output = match self.output_progress.take() {
            Some(StartupOutputProgress::Exclusive(output)) => output,
            Some(output @ StartupOutputProgress::ReaderBacked(_)) => {
                self.output_progress = Some(output);
                bail!(STARTUP_READER_ALREADY_INSTALLED_CONTEXT);
            }
            None => bail!(STARTUP_OUTPUT_PROGRESS_UNAVAILABLE_CONTEXT),
        };
        match reader.activate(output) {
            Ok(output) => {
                self.output_progress = Some(StartupOutputProgress::ReaderBacked(output));
                Ok(())
            }
            Err(failure) => {
                self.output_progress = Some(StartupOutputProgress::Exclusive(failure.output));
                Err(failure.error)
            }
        }
    }

    pub(crate) fn prepare_reader_activation(
        &mut self,
        reader: PendingPtyReader,
    ) -> Result<PreparedPtyReader> {
        match self.output_progress.as_mut() {
            Some(StartupOutputProgress::Exclusive(output)) => reader.prepare(output),
            Some(StartupOutputProgress::ReaderBacked(_)) => {
                bail!(STARTUP_READER_ALREADY_INSTALLED_CONTEXT)
            }
            None => bail!(STARTUP_OUTPUT_PROGRESS_UNAVAILABLE_CONTEXT),
        }
    }

    pub(crate) fn activate_prepared_reader(&mut self, reader: PreparedPtyReader) {
        let Some(StartupOutputProgress::Exclusive(output)) = self.output_progress.take() else {
            unreachable!("{PREPARED_READER_INVARIANT}")
        };
        let output = reader.activate_infallible(output);
        self.output_progress = Some(StartupOutputProgress::ReaderBacked(output));
    }

    pub(crate) fn terminate_with_owned_output(&mut self) -> Result<()> {
        let output = self
            .output_progress
            .take()
            .context(STARTUP_OUTPUT_PROGRESS_UNAVAILABLE_CONTEXT)?;
        let failure = self.stop_monitor().err();
        match output {
            StartupOutputProgress::Exclusive(output) => {
                self.terminate_with_exclusive_output(failure, output)
            }
            StartupOutputProgress::ReaderBacked(output) => {
                self.terminate_with_reader_output(failure, output)
            }
        }
    }

    pub(crate) fn terminate_with_exclusive_output(
        &mut self,
        mut failure: Option<anyhow::Error>,
        mut output: ExclusivePtyOutput,
    ) -> Result<()> {
        if self.tree.is_none() {
            return self.terminate_untracked_with_output(
                failure,
                StartupOutputProgress::Exclusive(output),
            );
        }
        let state_observer = self
            .fault_reaper
            .as_ref()
            .context(super::STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT)?
            .state_observer();
        let fault_reaper = self.reserve_deferred_output(output.supports_tracked_settlement())?;
        loop {
            match self.take_stopped_authority() {
                Ok((authority, tree)) => {
                    if let Some(reservation) = fault_reaper {
                        return self.defer_tracked_output(
                            authority,
                            tree,
                            StartupOutputProgress::Exclusive(output),
                            failure,
                            reservation,
                        );
                    }
                    let outcome = settle_inline_after_master_release(
                        authority,
                        tree,
                        output,
                        failure,
                        state_observer,
                    );
                    self.clear();
                    return outcome.fault.map_or(Ok(()), Err);
                }
                Err(error) => super::cleanup::remember(&mut failure, error),
            }
            self.progress_before_retry(&mut failure, || output.progress());
        }
    }

    pub(crate) fn terminate_with_reader_output(
        &mut self,
        mut failure: Option<anyhow::Error>,
        output: ReaderBackedPtyOutput,
    ) -> Result<()> {
        if self.tree.is_none() {
            return self.terminate_untracked_with_output(
                failure,
                StartupOutputProgress::ReaderBacked(output),
            );
        }
        let state_observer = self
            .fault_reaper
            .as_ref()
            .context(super::STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT)?
            .state_observer();
        let fault_reaper = self.reserve_deferred_output(output.supports_tracked_settlement())?;
        let mut recovery = output.recovery();
        loop {
            match self.take_stopped_authority() {
                Ok((authority, tree)) => {
                    drop(recovery);
                    if let Some(reservation) = fault_reaper {
                        return self.defer_tracked_output(
                            authority,
                            tree,
                            StartupOutputProgress::ReaderBacked(output),
                            failure,
                            reservation,
                        );
                    }
                    let outcome = settle_inline(authority, tree, output, failure, state_observer);
                    self.clear();
                    return outcome.fault.map_or(Ok(()), Err);
                }
                Err(error) => super::cleanup::remember(&mut failure, error),
            }
            self.progress_before_retry(&mut failure, || recovery.progress());
        }
    }

    pub(crate) fn progress_before_retry<Progress>(
        &mut self,
        failure: &mut Option<anyhow::Error>,
        mut progress: Progress,
    ) where
        Progress: FnMut() -> Result<OutputProgressState>,
    {
        if let Err(error) = progress() {
            super::cleanup::remember(failure, error);
        }
        std::thread::sleep(STARTUP_CHILD_EXIT_POLL_INTERVAL);
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_guard_output_adversarial_tests.rs"]
mod adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_guard_output_branch_tests.rs"]
mod tests;
