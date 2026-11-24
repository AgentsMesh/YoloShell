#[path = "runtime_artifacts_api.rs"]
mod artifacts;
#[path = "runtime_outbox_api.rs"]
mod outbox;
#[path = "runtime_outbox_state_api.rs"]
mod outbox_state;

use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::{CodexHookIntake, CodexRetirementDisposition, CodexRetirementRecord};
use anyhow::Result;

impl CodexHookIntake {
    pub fn persist_for_test(&self, envelope: HookEnvelope) -> Result<()> {
        self.persist(envelope)
    }

    pub fn persist_occurrence_for_test(&self, envelope: HookEnvelope) -> Result<HookEnvelope> {
        self.persist_occurrence(envelope)
    }
}

impl CodexRetirementRecord {
    pub fn disposition_for_test(&self) -> CodexRetirementDisposition {
        self.disposition()
    }
}
