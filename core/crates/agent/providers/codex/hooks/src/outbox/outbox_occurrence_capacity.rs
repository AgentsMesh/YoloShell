use super::constants::errors;
use super::{CodexOccurrenceIdentity, CodexOutbox};
use crate::agent::hooks::{EventSequence, HookEnvelope};
use anyhow::{Result, ensure};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexOccurrenceBindingCapacity {
    Available,
    RecoveryRequired,
}

impl CodexOutbox {
    pub(crate) fn occurrence_binding_capacity(
        &self,
        occurrence: &HookEnvelope,
        identity: &CodexOccurrenceIdentity,
    ) -> Result<CodexOccurrenceBindingCapacity> {
        ensure!(
            !occurrence.event_id.is_nil() && occurrence.sequence == EventSequence::ZERO,
            errors::OUTBOX_CORRUPT
        );
        identity
            .monotonic_nanos_for(occurrence)
            .map_err(|_| anyhow::anyhow!(errors::OUTBOX_CORRUPT))?;
        self.with_locked_state(|state| {
            if super::outbox_occurrence_state::find_replay(state, occurrence, Some(identity))?
                .is_some()
            {
                return Ok(CodexOccurrenceBindingCapacity::Available);
            }
            Ok(if super::outbox_occurrence_state::has_capacity(state)? {
                CodexOccurrenceBindingCapacity::Available
            } else {
                CodexOccurrenceBindingCapacity::RecoveryRequired
            })
        })
    }

    pub(crate) fn ensure_occurrence_binding_capacity(
        &self,
        occurrence: &HookEnvelope,
        identity: &CodexOccurrenceIdentity,
    ) -> Result<()> {
        ensure!(
            self.occurrence_binding_capacity(occurrence, identity)?
                == CodexOccurrenceBindingCapacity::Available,
            errors::OUTBOX_FULL
        );
        Ok(())
    }
}
