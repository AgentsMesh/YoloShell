use super::constants::errors;
use super::intake_constants::*;
use crate::agent::hooks::{EventSequence, HookEnvelope, HookIdentity};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodexIntakeRecord {
    schema_version: u16,
    occurrence_id: Uuid,
    observed_monotonic_nanos: u64,
    envelope: HookEnvelope,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CodexIntakeQuarantine {
    schema_version: u16,
    pending_file: String,
    pending_identity: crate::paths::identity::FileIdentity,
}

impl CodexIntakeRecord {
    pub(super) fn capture_explicit(envelope: HookEnvelope) -> Result<Self> {
        ensure!(
            !envelope.event_id.is_nil() && envelope.sequence == EventSequence::ZERO,
            errors::INTAKE_CORRUPT
        );
        let occurrence_id = envelope.event_id;
        let observed = crate::process::monotonic_time_ns()
            .map_err(|error| error.context(errors::INTAKE_CLOCK))?;
        Self::capture_at(envelope, occurrence_id, observed)
    }

    pub(super) fn capture(envelope: HookEnvelope) -> Result<Self> {
        ensure!(
            envelope.event_id.is_nil() && envelope.sequence == EventSequence::ZERO,
            errors::INTAKE_CORRUPT
        );
        let occurrence_id = Uuid::new_v4();
        let observed_monotonic_nanos = crate::process::monotonic_time_ns()
            .map_err(|error| error.context(errors::INTAKE_CLOCK))?;
        Self::capture_at(envelope, occurrence_id, observed_monotonic_nanos)
    }

    fn capture_at(
        mut envelope: HookEnvelope,
        occurrence_id: Uuid,
        observed_monotonic_nanos: u64,
    ) -> Result<Self> {
        envelope.event_id = occurrence_id;
        let record = Self {
            schema_version: RECORD_SCHEMA_VERSION,
            occurrence_id,
            observed_monotonic_nanos,
            envelope,
        };
        record.validate(
            record.pending_file_name().as_str(),
            &record.envelope.identity,
        )?;
        Ok(record)
    }

    pub(super) fn validate(&self, file_name: &str, owner: &HookIdentity) -> Result<()> {
        ensure!(
            self.schema_version == RECORD_SCHEMA_VERSION
                && !self.occurrence_id.is_nil()
                && self.envelope.event_id == self.occurrence_id
                && self.envelope.sequence == EventSequence::ZERO
                && &self.envelope.identity == owner
                && file_name == self.pending_file_name(),
            errors::INTAKE_CORRUPT
        );
        super::intake_envelope_validation::validate(&self.envelope)
            .map_err(|_| anyhow::anyhow!(errors::INTAKE_CORRUPT))
    }

    pub(super) fn pending_file_name(&self) -> String {
        pending_file_name(self.observed_monotonic_nanos, self.occurrence_id)
    }

    pub(crate) fn ordering_key(&self) -> (u64, Uuid) {
        (self.observed_monotonic_nanos, self.occurrence_id)
    }

    pub(crate) fn envelope(&self) -> &HookEnvelope {
        &self.envelope
    }

    #[cfg(test)]
    pub(crate) fn for_test(
        envelope: HookEnvelope,
        occurrence_id: Uuid,
        observed_monotonic_nanos: u64,
    ) -> Result<Self> {
        ensure!(
            envelope.event_id.is_nil() && envelope.sequence == EventSequence::ZERO,
            errors::INTAKE_CORRUPT
        );
        Self::capture_at(envelope, occurrence_id, observed_monotonic_nanos)
    }
}

impl CodexIntakeQuarantine {
    pub(super) fn new(
        pending_file: String,
        pending_identity: crate::paths::identity::FileIdentity,
    ) -> Self {
        Self {
            schema_version: QUARANTINE_SCHEMA_VERSION,
            pending_file,
            pending_identity,
        }
    }

    pub(super) fn validate(
        &self,
        pending_file: &str,
        pending_identity: crate::paths::identity::FileIdentity,
    ) -> Result<()> {
        ensure!(
            self.schema_version == QUARANTINE_SCHEMA_VERSION
                && self.pending_file == pending_file
                && self.pending_identity == pending_identity,
            errors::INTAKE_CORRUPT
        );
        Ok(())
    }
}

pub(super) fn is_pending_file(name: &str) -> bool {
    pending_file_key(name).is_some()
}

pub(super) fn pending_file_key(name: &str) -> Option<(u64, Uuid)> {
    let Some(stem) = name.strip_suffix(EVENT_SUFFIX) else {
        return None;
    };
    let Some((timestamp, occurrence)) = stem.split_once(NAME_SEPARATOR) else {
        return None;
    };
    let Ok(parsed_timestamp) = timestamp.parse::<u64>() else {
        return None;
    };
    let Ok(parsed_occurrence) = Uuid::parse_str(occurrence) else {
        return None;
    };
    (timestamp == parsed_timestamp.to_string()
        && !parsed_occurrence.is_nil()
        && occurrence == parsed_occurrence.hyphenated().to_string())
    .then_some((parsed_timestamp, parsed_occurrence))
}

pub(super) fn pending_file_name(monotonic_nanos: u64, occurrence_id: Uuid) -> String {
    format!("{monotonic_nanos}{NAME_SEPARATOR}{occurrence_id}{EVENT_SUFFIX}")
}

pub(super) fn quarantine_file_name(pending_file: &str) -> String {
    format!("{pending_file}{QUARANTINE_SUFFIX}")
}
