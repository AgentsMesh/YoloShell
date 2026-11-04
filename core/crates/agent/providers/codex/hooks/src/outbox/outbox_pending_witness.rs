use super::CodexOccurrenceIdentity;
use super::constants::{MAX_OCCURRENCE_BINDING_BYTES, errors};
use super::outbox_occurrence_binding::CodexOccurrenceBinding;
use crate::agent::hooks::{EventSequence, HookEnvelope};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct CodexPendingOccurrenceWitness(CodexOccurrenceBinding);

impl CodexPendingOccurrenceWitness {
    pub(super) fn capture(
        assigned: &HookEnvelope,
        occurrence: &HookEnvelope,
        identity: &CodexOccurrenceIdentity,
    ) -> Result<Self> {
        let witness = Self(CodexOccurrenceBinding::capture(
            assigned, occurrence, identity,
        )?);
        witness.validate()?;
        Ok(witness)
    }

    pub(super) fn assign(
        &self,
        incoming: &HookEnvelope,
        identity: &CodexOccurrenceIdentity,
    ) -> Result<HookEnvelope> {
        self.0.assign(incoming, identity)
    }

    pub(super) fn assign_envelope(&self, incoming: &HookEnvelope) -> Result<HookEnvelope> {
        self.0.assign_envelope(incoming)
    }

    pub(super) fn event_id(&self) -> uuid::Uuid {
        self.0.event_id()
    }

    pub(super) fn sequence(&self) -> EventSequence {
        self.0.sequence()
    }

    pub(super) fn into_binding(self) -> CodexOccurrenceBinding {
        self.0
    }

    pub(super) fn validate(&self) -> Result<()> {
        self.0.validate()?;
        let encoded = serde_json::to_vec(self).context(errors::OUTBOX_CORRUPT)?;
        ensure!(
            encoded.len() <= MAX_OCCURRENCE_BINDING_BYTES,
            errors::OUTBOX_CORRUPT
        );
        Ok(())
    }
}
