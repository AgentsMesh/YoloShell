use super::super::super::error::CodexHookRejection;
use super::CodexLiveProducerProof;
use crate::codex_hooks::CodexRetirementWriterFence;

impl CodexLiveProducerProof {
    pub(in crate::worker::codex_hook) fn verify_writer(&self) -> Result<(), CodexHookRejection> {
        self.writer
            .as_ref()
            .ok_or(CodexHookRejection::Internal)?
            .verify_unsealed()
            .map_err(|error| CodexHookRejection::Storage(error))
    }

    pub(in crate::worker::codex_hook) fn take_retirement_writer(
        &mut self,
    ) -> Result<CodexRetirementWriterFence, CodexHookRejection> {
        self.writer.take().ok_or(CodexHookRejection::Internal)
    }
}
