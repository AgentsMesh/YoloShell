use super::constants::errors;
use crate::agent::hooks::{EventSequence, HookEnvelope};
use crate::paths::identity::FileIdentity;
use anyhow::{Result, ensure};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexOccurrenceIdentity {
    file_name: String,
    file_identity: FileIdentity,
}

impl CodexOccurrenceIdentity {
    pub(super) fn capture(
        file_name: &str,
        file_identity: FileIdentity,
        envelope: &HookEnvelope,
    ) -> Result<Self> {
        let identity = Self {
            file_name: file_name.to_owned(),
            file_identity,
        };
        identity.monotonic_nanos_for(envelope)?;
        Ok(identity)
    }

    pub(super) fn file_name(&self) -> &str {
        &self.file_name
    }

    pub(super) fn file_identity(&self) -> FileIdentity {
        self.file_identity
    }

    pub(super) fn monotonic_nanos_for(&self, envelope: &HookEnvelope) -> Result<u64> {
        ensure!(
            !envelope.event_id.is_nil() && envelope.sequence == EventSequence::ZERO,
            errors::INTAKE_CORRUPT
        );
        let (monotonic_nanos, event_id) = super::intake_record::pending_file_key(&self.file_name)
            .ok_or_else(|| anyhow::anyhow!(errors::INTAKE_CORRUPT))?;
        ensure!(event_id == envelope.event_id, errors::INTAKE_CORRUPT);
        Ok(monotonic_nanos)
    }
}
