use super::CodexHookRuntime;
use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::CodexHookIntake;
use anyhow::{Result, ensure};

const NATIVE_OCCURRENCE_CONFLICT: &str =
    "native Codex callback occurrence conflicts with its durable history";

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn persist_native_event(
        &self,
        event: HookEnvelope,
    ) -> Result<HookEnvelope> {
        ensure!(event.identity == self.owner, NATIVE_OCCURRENCE_CONFLICT);
        let intake = CodexHookIntake::for_identity(self.artifacts.intake_directory(), &self.owner)?;
        if let Some(existing) = intake.find_exact_occurrence(&event)? {
            return Ok(existing.envelope().clone());
        }
        let state = self.outbox.load()?;
        if let Some(retained) = state
            .retained_events()
            .find(|retained| retained.event_id == event.event_id)
        {
            let mut normalized = retained.clone();
            normalized.sequence = event.sequence;
            ensure!(normalized == event, NATIVE_OCCURRENCE_CONFLICT);
            return Ok(event);
        }
        intake.persist_explicit_occurrence(event)
    }
}
