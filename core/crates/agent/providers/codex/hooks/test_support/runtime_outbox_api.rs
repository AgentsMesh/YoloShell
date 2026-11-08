use crate::agent::hooks::{HookEnvelope, HookViolation};
use crate::codex_hooks::{
    CodexDurableRejection, CodexEnqueueAdmission, CodexOccurrenceIdentity, CodexOutbox,
};
use anyhow::Result;

impl CodexOutbox {
    pub fn enqueue_admitted_for_test<T>(
        &self,
        envelope: HookEnvelope,
        occurrence_identity: Option<&CodexOccurrenceIdentity>,
        admit: impl FnOnce(&HookEnvelope, CodexEnqueueAdmission) -> Result<T>,
        commit_fence: impl FnMut(&HookEnvelope, CodexEnqueueAdmission, &T) -> Result<()>,
    ) -> Result<(HookEnvelope, T, Option<CodexDurableRejection>)> {
        self.enqueue_admitted(envelope, occurrence_identity, admit, commit_fence)
            .map(|result| result.into_parts())
    }
}

impl CodexDurableRejection {
    pub fn capture_for_test(event: &HookEnvelope, violation: HookViolation) -> Result<Self> {
        Self::capture(event, violation)
    }
}
