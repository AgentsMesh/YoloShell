use crate::identity::{
    EventSequence, HookIdentity, ProcessIdentity, HookProviderKind, ProviderVersion,
};
use crate::reducer::Reducer;
use crate::revocation::predecessor::PreRevocationProof;
use crate::revocation::process::ProcessObservationRevocationProof;
use crate::revocation::{HookEventRevocationProof, RevocationEvidence};
use crate::snapshot::event::wire::EventEnvelopeDto;
use crate::snapshot::wire::RevocationDto;
use crate::snapshot::{ReducerSnapshot, SnapshotViolation, document_from_state};

pub(crate) fn to_dto(evidence: Option<&RevocationEvidence>) -> Option<RevocationDto> {
    evidence.map(|evidence| match evidence {
        RevocationEvidence::HookEvent {
            predecessor,
            event,
            proof,
        } => RevocationDto::HookEvent {
            predecessor: Box::new(document_from_state(
                predecessor.state(),
                predecessor.revision(),
            )),
            predecessor_sha256: predecessor.snapshot_sha256(),
            event: EventEnvelopeDto::from(event),
            event_id: proof.event_id(),
            sequence: proof.sequence().0,
            violation: proof.violation(),
            envelope_sha256: proof.envelope_sha256(),
        },
        RevocationEvidence::ProcessObservation {
            predecessor,
            observed,
            violation,
            proof,
        } => RevocationDto::ProcessObservation {
            predecessor: Box::new(document_from_state(
                predecessor.state(),
                predecessor.revision(),
            )),
            predecessor_sha256: predecessor.snapshot_sha256(),
            pid: observed.pid,
            start_time: observed.start_time,
            violation: *violation,
            observation_sha256: proof.digest(),
        },
    })
}

pub(crate) fn from_dto(
    evidence: Option<RevocationDto>,
    identity: &HookIdentity,
    provider: &HookProviderKind,
    version: ProviderVersion,
) -> Result<Option<RevocationEvidence>, SnapshotViolation> {
    evidence
        .map(|evidence| restore(evidence, identity, provider, version))
        .transpose()
}

fn restore(
    evidence: RevocationDto,
    identity: &HookIdentity,
    provider: &HookProviderKind,
    version: ProviderVersion,
) -> Result<RevocationEvidence, SnapshotViolation> {
    Ok(match evidence {
        RevocationDto::HookEvent {
            predecessor,
            predecessor_sha256,
            event,
            event_id,
            sequence,
            violation,
            envelope_sha256,
        } => RevocationEvidence::restored_hook(
            restore_predecessor(
                *predecessor,
                predecessor_sha256,
                identity,
                provider,
                version,
            )?,
            event.into(),
            HookEventRevocationProof::restored(
                event_id,
                EventSequence(sequence),
                violation,
                envelope_sha256,
            ),
        ),
        RevocationDto::ProcessObservation {
            predecessor,
            predecessor_sha256,
            pid,
            start_time,
            violation,
            observation_sha256,
        } => RevocationEvidence::restored_process(
            restore_predecessor(
                *predecessor,
                predecessor_sha256,
                identity,
                provider,
                version,
            )?,
            ProcessIdentity::new(pid, start_time),
            violation,
            ProcessObservationRevocationProof::restored(observation_sha256),
        ),
    })
}

fn restore_predecessor(
    document: crate::snapshot::wire::SnapshotDocument,
    snapshot_sha256: crate::sha256_digest::Sha256Digest,
    identity: &HookIdentity,
    provider: &HookProviderKind,
    version: ProviderVersion,
) -> Result<PreRevocationProof, SnapshotViolation> {
    let reducer = Reducer::restore(
        ReducerSnapshot::from_document(document),
        identity.clone(),
        provider.clone(),
        version,
    )?;
    PreRevocationProof::restored(reducer, snapshot_sha256)
}
