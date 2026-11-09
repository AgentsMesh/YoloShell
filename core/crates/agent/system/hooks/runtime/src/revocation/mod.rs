pub(crate) mod predecessor;
pub(crate) mod process;

use crate::revocation::predecessor::PreRevocationProof;
use crate::revocation::process::ProcessObservationRevocationProof;
use crate::sha256_digest::Sha256Digest;
use crate::{EventSequence, HookEnvelope, HookViolation, ProcessIdentity};
use serde::{Deserialize, Serialize};

const ENVELOPE_DIGEST_DOMAIN: &str = "yolo.agent-hook.revocation-envelope.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HookEventRevocationProof {
    event_id: uuid::Uuid,
    sequence: EventSequence,
    violation: HookViolation,
    envelope_sha256: Sha256Digest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevocationEvidence {
    HookEvent {
        predecessor: PreRevocationProof,
        event: HookEnvelope,
        proof: HookEventRevocationProof,
    },
    ProcessObservation {
        predecessor: PreRevocationProof,
        observed: ProcessIdentity,
        violation: HookViolation,
        proof: ProcessObservationRevocationProof,
    },
}

impl RevocationEvidence {
    pub(crate) fn hook(
        predecessor: PreRevocationProof,
        event: HookEnvelope,
        violation: HookViolation,
    ) -> Result<Self, HookViolation> {
        let proof = HookEventRevocationProof::capture(&event, violation)?;
        Ok(Self::HookEvent {
            predecessor,
            event,
            proof,
        })
    }

    pub(crate) fn restored_hook(
        predecessor: PreRevocationProof,
        event: HookEnvelope,
        proof: HookEventRevocationProof,
    ) -> Self {
        Self::HookEvent {
            predecessor,
            event,
            proof,
        }
    }

    pub(crate) fn process(
        predecessor: PreRevocationProof,
        observed: ProcessIdentity,
        violation: HookViolation,
    ) -> Result<Self, HookViolation> {
        let proof = ProcessObservationRevocationProof::capture(observed, violation)?;
        Ok(Self::ProcessObservation {
            predecessor,
            observed,
            violation,
            proof,
        })
    }

    pub(crate) fn restored_process(
        predecessor: PreRevocationProof,
        observed: ProcessIdentity,
        violation: HookViolation,
        proof: ProcessObservationRevocationProof,
    ) -> Self {
        Self::ProcessObservation {
            predecessor,
            observed,
            violation,
            proof,
        }
    }

    pub fn hook_rejection(&self) -> Option<(&HookEnvelope, HookEventRevocationProof)> {
        match self {
            Self::HookEvent { event, proof, .. } => Some((event, *proof)),
            Self::ProcessObservation { .. } => None,
        }
    }

    pub const fn is_process_observation(&self) -> bool {
        matches!(self, Self::ProcessObservation { .. })
    }

    pub(crate) fn is_valid_for(&self, expected_process: ProcessIdentity) -> bool {
        if !self.predecessor().matches_snapshot() {
            return false;
        }
        match self {
            Self::HookEvent { event, proof, .. } => proof.matches(event),
            Self::ProcessObservation {
                observed,
                violation,
                proof,
                ..
            } => {
                observed.is_valid()
                    && *observed != expected_process
                    && *violation == HookViolation::ProcessMismatch
                    && proof.matches(*observed, *violation)
            }
        }
    }

    pub(crate) const fn predecessor(&self) -> &PreRevocationProof {
        match self {
            Self::HookEvent { predecessor, .. } | Self::ProcessObservation { predecessor, .. } => {
                predecessor
            }
        }
    }
}

impl HookEventRevocationProof {
    pub fn capture(event: &HookEnvelope, violation: HookViolation) -> Result<Self, HookViolation> {
        if !violation.is_durable_rejection() {
            return Err(HookViolation::InvalidEnvelope);
        }
        let envelope_sha256 = envelope_digest(event)?;
        Ok(Self {
            event_id: event.event_id,
            sequence: event.sequence,
            violation,
            envelope_sha256,
        })
    }

    pub fn matches(self, event: &HookEnvelope) -> bool {
        self.event_id == event.event_id
            && self.sequence == event.sequence
            && self.violation.is_durable_rejection()
            && event.has_bounded_storage_encoding()
            && envelope_digest(event).is_ok_and(|digest| digest == self.envelope_sha256)
    }

    pub const fn event_id(self) -> uuid::Uuid {
        self.event_id
    }

    pub const fn sequence(self) -> EventSequence {
        self.sequence
    }

    pub const fn violation(self) -> HookViolation {
        self.violation
    }

    pub const fn envelope_sha256(self) -> Sha256Digest {
        self.envelope_sha256
    }

    pub(crate) const fn restored(
        event_id: uuid::Uuid,
        sequence: EventSequence,
        violation: HookViolation,
        envelope_sha256: Sha256Digest,
    ) -> Self {
        Self {
            event_id,
            sequence,
            violation,
            envelope_sha256,
        }
    }
}

fn envelope_digest(event: &HookEnvelope) -> Result<Sha256Digest, HookViolation> {
    Sha256Digest::capture(&(ENVELOPE_DIGEST_DOMAIN, event))
        .map_err(|_| HookViolation::InvalidEnvelope)
}
