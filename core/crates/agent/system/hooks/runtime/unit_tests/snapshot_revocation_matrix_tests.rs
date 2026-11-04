use super::*;
use crate::revocation::predecessor::PreRevocationProof;
use crate::test_support::{
    CLAUDE_VERSION, CODEX_VERSION, claude_envelope, claude_started, identity,
};
use crate::{EventKind, HookEventRevocationProof, HookViolation, HookProviderKind};

const SEQUENCE_GAP: u64 = 2;

fn revoked() -> Reducer {
    let identity = identity();
    let mut reducer = Reducer::new(identity.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    claude_started(&mut reducer, &identity);
    let event = claude_envelope(
        &identity,
        crate::FIRST_SEQUENCE + SEQUENCE_GAP,
        EventKind::UserPromptSubmit,
    );
    assert_eq!(reducer.apply(event), Err(HookViolation::SequenceGap));
    reducer.snapshot().unwrap();
    reducer
}

#[test]
fn revocation_predecessor_must_match_every_top_level_provider_and_identity_dimension() {
    let original = revoked();
    let evidence = original.state.revocation.as_ref().unwrap();
    validate(&original.state, original.revision, evidence).unwrap();
    let mutations: &[(&str, fn(&mut NormalizedLifecycle))] = &[
        (stringify!(identity), |value| {
            value.expected_identity.runtime_epoch = uuid::Uuid::new_v4()
        }),
        (stringify!(provider), |value| {
            value.provider = HookProviderKind::Codex
        }),
        (stringify!(version), |value| {
            value.provider_version = CODEX_VERSION
        }),
        (stringify!(state), |value| {
            value.turn = crate::TurnState::Running
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.state.clone();
        mutate(&mut changed);
        assert_eq!(
            validate(&changed, original.revision, evidence),
            Err(SnapshotViolation::InvalidState),
            "{field}"
        );
    }
    let mut revision = original.revision;
    revision.advance().unwrap();
    assert_eq!(
        validate(&original.state, revision, evidence),
        Err(SnapshotViolation::InvalidState)
    );
}

#[test]
fn a_revoked_or_revocation_bearing_predecessor_cannot_supply_a_new_revocation() {
    let original = revoked();
    let RevocationEvidence::HookEvent { event, proof, .. } =
        original.state.revocation.as_ref().unwrap()
    else {
        unreachable!()
    };
    for keep_revoked in [true, false] {
        let mut prior = original.clone();
        prior.state.revoked = keep_revoked;
        let evidence = RevocationEvidence::HookEvent {
            predecessor: PreRevocationProof::capture(&prior).unwrap(),
            event: event.clone(),
            proof: *proof,
        };
        assert_eq!(
            validate(&original.state, original.revision, &evidence),
            Err(SnapshotViolation::InvalidState)
        );
    }
}

#[test]
fn revocation_replay_must_reproduce_the_recorded_rejection_not_a_success_or_other_error() {
    let original = revoked();
    let RevocationEvidence::HookEvent {
        predecessor,
        event,
        proof,
    } = original.state.revocation.as_ref().unwrap()
    else {
        unreachable!()
    };
    let wrong = HookEventRevocationProof::capture(event, HookViolation::InvalidIdentity).unwrap();
    let changed = RevocationEvidence::HookEvent {
        predecessor: predecessor.clone(),
        event: event.clone(),
        proof: wrong,
    };
    assert_eq!(
        replay_rejection(&mut predecessor.replay(), &changed),
        Err(SnapshotViolation::InvalidState)
    );
    let mut accepted = event.clone();
    accepted.sequence = predecessor.state().last_sequence.next().unwrap();
    let changed = RevocationEvidence::HookEvent {
        predecessor: predecessor.clone(),
        event: accepted,
        proof: *proof,
    };
    assert_eq!(
        replay_rejection(&mut predecessor.replay(), &changed),
        Err(SnapshotViolation::InvalidState)
    );
}
