use super::{OutputDescriptor, OutputProgressState, PtyOutputRecovery};
use anyhow::Result;

impl PtyOutputRecovery {
    pub(crate) fn report_tracked_producers_gone(
        &mut self,
        proof: &crate::process::TrackedProducersGone,
    ) -> Result<()> {
        if let Some(lifecycle) = self.reader_lifecycle.as_ref() {
            lifecycle.mark_tracked_producers_gone(proof);
        }
        Ok(())
    }

    pub(crate) fn mark_untracked_producers_gone(
        &mut self,
        proof: crate::worker::startup::startup_untracked_proof::UntrackedStartupProducersGone,
    ) -> Result<OutputProgressState> {
        if let Some(lifecycle) = self.reader_lifecycle.as_ref() {
            lifecycle.mark_untracked_producers_gone(proof);
        }
        self.progress()
    }

    pub(crate) fn progress(&mut self) -> Result<OutputProgressState> {
        match &*self.descriptor.lock() {
            OutputDescriptor::Native(file) => crate::worker::pty_output_terminal::resume(file)?,
            OutputDescriptor::Released => return Ok(OutputProgressState::Complete),
            #[cfg(test)]
            OutputDescriptor::Observed(observed) | OutputDescriptor::Quiescent(observed) => {
                observed.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            }
        }
        Ok(self
            .reader_lifecycle
            .as_ref()
            .map(|lifecycle| lifecycle.output_progress_state())
            .unwrap_or(OutputProgressState::Complete))
    }
}
