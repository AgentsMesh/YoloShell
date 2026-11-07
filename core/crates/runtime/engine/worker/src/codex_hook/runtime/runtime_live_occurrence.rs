use super::super::constants::CODEX_DURABLE_EVENT_ERROR;
use super::CodexHookRuntime;
use crate::agent::hooks::{EventSequence, HookEnvelope};
use crate::codex_hooks::{CodexHookIntake, CodexIntakeOccurrence};
use anyhow::{Context, Result};

pub(super) struct VerifiedLiveOccurrence {
    intake: CodexHookIntake,
    occurrence: CodexIntakeOccurrence,
}

impl VerifiedLiveOccurrence {
    pub(super) fn required(runtime: &CodexHookRuntime, event: &HookEnvelope) -> Result<Self> {
        Self::optional(runtime, event)?.context(CODEX_DURABLE_EVENT_ERROR)
    }

    pub(super) fn optional(
        runtime: &CodexHookRuntime,
        event: &HookEnvelope,
    ) -> Result<Option<Self>> {
        let intake =
            CodexHookIntake::for_identity(runtime.artifacts.intake_directory(), &event.identity)?;
        let mut expected = event.clone();
        expected.sequence = EventSequence::ZERO;
        let occurrence = intake.find_exact_occurrence(&expected)?;
        Ok(occurrence.map(|occurrence| Self { intake, occurrence }))
    }

    pub(super) fn verify(&self) -> Result<()> {
        self.intake.verify_current(&self.occurrence)
    }

    pub(super) fn envelope(&self) -> &HookEnvelope {
        self.occurrence.envelope()
    }

    pub(super) fn durable_identity(&self) -> Result<crate::codex_hooks::CodexOccurrenceIdentity> {
        self.occurrence.durable_identity()
    }

    pub(super) fn authoritative_outcome(
        &self,
        outbox: &crate::codex_hooks::CodexOutbox,
    ) -> Result<Option<crate::codex_hooks::CodexAuthoritativeOccurrence>> {
        self.verify()?;
        let outcome = outbox.authoritative_intake_outcome(&self.occurrence)?;
        self.verify()?;
        Ok(outcome)
    }

    pub(super) fn complete(self) -> Result<()> {
        self.verify()?;
        self.intake.complete(&self.occurrence)
    }

    pub(super) fn complete_with_fence<F>(self, fence: F) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        self.verify()?;
        self.intake.complete_with_fence(&self.occurrence, fence)
    }
}
