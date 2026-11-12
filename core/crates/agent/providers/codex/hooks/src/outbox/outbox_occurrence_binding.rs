use super::CodexOccurrenceIdentity;
use super::constants::errors;
use crate::agent::hooks::{EventSequence, HookEnvelope};
use crate::sha256_digest::Sha256Digest;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

const ENVELOPE_DIGEST_DOMAIN: &str = "yolo.codex-hook.occurrence-envelope.v1";
const ARTIFACT_DIGEST_DOMAIN: &str = "yolo.codex-hook.occurrence-artifact.v2";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(super) struct CodexOccurrenceBinding(
    uuid::Uuid,
    EventSequence,
    u64,
    #[serde(with = "crate::sha256_digest::compact")] Sha256Digest,
    #[serde(with = "crate::sha256_digest::compact")] Sha256Digest,
);

impl CodexOccurrenceBinding {
    pub(super) fn capture(
        assigned: &HookEnvelope,
        occurrence: &HookEnvelope,
        identity: &CodexOccurrenceIdentity,
    ) -> Result<Self> {
        ensure!(
            assigned.sequence != EventSequence::ZERO
                && occurrence.sequence == EventSequence::ZERO
                && super::outbox_validation::same_event(assigned, occurrence),
            errors::OUTBOX_CORRUPT
        );
        let monotonic_nanos = identity
            .monotonic_nanos_for(occurrence)
            .context(errors::OUTBOX_CORRUPT)?;
        let envelope_sha256 = envelope_digest(occurrence)?;
        let artifact_sha256 = artifact_digest(occurrence, identity, envelope_sha256)?;
        let binding = Self(
            assigned.event_id,
            assigned.sequence,
            monotonic_nanos,
            envelope_sha256,
            artifact_sha256,
        );
        binding.validate()?;
        Ok(binding)
    }

    pub(super) fn assign(
        &self,
        incoming: &HookEnvelope,
        identity: &CodexOccurrenceIdentity,
    ) -> Result<HookEnvelope> {
        let mut occurrence = self.assign_envelope(incoming)?;
        occurrence.sequence = EventSequence::ZERO;
        ensure!(
            identity
                .monotonic_nanos_for(&occurrence)
                .context(errors::OUTBOX_CORRUPT)?
                == self.2
                && artifact_digest(&occurrence, identity, self.3)? == self.4,
            errors::OUTBOX_CORRUPT
        );
        occurrence.sequence = self.1;
        Ok(occurrence)
    }

    pub(super) fn assign_envelope(&self, incoming: &HookEnvelope) -> Result<HookEnvelope> {
        ensure!(incoming.event_id == self.0, errors::OUTBOX_CORRUPT);
        ensure!(
            incoming.sequence == EventSequence::ZERO || incoming.sequence == self.1,
            errors::OUTBOX_CORRUPT
        );
        let mut occurrence = incoming.clone();
        occurrence.sequence = EventSequence::ZERO;
        ensure!(
            envelope_digest(&occurrence)? == self.3,
            errors::OUTBOX_CORRUPT
        );
        occurrence.sequence = self.1;
        Ok(occurrence)
    }

    pub(super) fn event_id(&self) -> uuid::Uuid {
        self.0
    }

    pub(super) fn sequence(&self) -> EventSequence {
        self.1
    }

    pub(super) fn file_name(&self) -> String {
        super::intake_record::pending_file_name(self.2, self.0)
    }

    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            !self.0.is_nil() && self.1 != EventSequence::ZERO,
            errors::OUTBOX_CORRUPT
        );
        Ok(())
    }
}

fn envelope_digest(occurrence: &HookEnvelope) -> Result<Sha256Digest> {
    ensure!(
        !occurrence.event_id.is_nil() && occurrence.sequence == EventSequence::ZERO,
        errors::OUTBOX_CORRUPT
    );
    Sha256Digest::capture(&(ENVELOPE_DIGEST_DOMAIN, occurrence)).context(errors::OUTBOX_CORRUPT)
}

fn artifact_digest(
    occurrence: &HookEnvelope,
    identity: &CodexOccurrenceIdentity,
    envelope_sha256: Sha256Digest,
) -> Result<Sha256Digest> {
    identity
        .monotonic_nanos_for(occurrence)
        .context(errors::OUTBOX_CORRUPT)?;
    Sha256Digest::capture(&(
        ARTIFACT_DIGEST_DOMAIN,
        envelope_sha256,
        identity.file_name(),
        identity.file_identity().content_version_evidence(),
    ))
    .context(errors::OUTBOX_CORRUPT)
}
