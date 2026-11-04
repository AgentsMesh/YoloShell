use crate::reducer::Reducer;
use crate::revocation::RevocationEvidence;
use crate::snapshot::SnapshotViolation;
use crate::snapshot::revision::AuthorityRevision;
use crate::state::NormalizedLifecycle;

pub(crate) fn validate(
    state: &NormalizedLifecycle,
    revision: AuthorityRevision,
    evidence: &RevocationEvidence,
) -> Result<(), SnapshotViolation> {
    let predecessor = evidence.predecessor();
    let prior = predecessor.state();
    if prior.revoked
        || prior.revocation.is_some()
        || prior.expected_identity != state.expected_identity
        || prior.provider != state.provider
        || prior.provider_version != state.provider_version
    {
        return Err(SnapshotViolation::InvalidState);
    }
    crate::snapshot::validation::validate_lifecycle(prior)?;
    crate::snapshot::reachability::validate_reachability(prior, predecessor.revision())?;
    predecessor.revision().validate_for(prior)?;

    let mut replay = predecessor.replay();
    replay_rejection(&mut replay, evidence)?;
    if replay.state != *state || replay.revision != revision {
        return Err(SnapshotViolation::InvalidState);
    }
    Ok(())
}

fn replay_rejection(
    replay: &mut Reducer,
    evidence: &RevocationEvidence,
) -> Result<(), SnapshotViolation> {
    let accepted = match evidence {
        RevocationEvidence::HookEvent { event, proof, .. } => {
            matches!(replay.apply(event.clone()), Err(actual) if actual == proof.violation())
        }
        RevocationEvidence::ProcessObservation {
            observed,
            violation,
            ..
        } => matches!(
            replay.process_exit_with_identity(*observed),
            Err(actual) if actual == *violation
        ),
    };
    accepted
        .then_some(())
        .ok_or(SnapshotViolation::InvalidState)
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/snapshot_revocation_matrix_tests.rs"]
mod matrix_tests;
