use super::snapshot::revision::AuthorityRevision;
use super::test_support::{CODEX_VERSION, envelope, identity};
use super::*;

fn exhausted_reducer() -> (HookIdentity, Reducer) {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    reducer.revision = AuthorityRevision::last_usable();
    (expected, reducer)
}

#[test]
fn accepted_event_cannot_mutate_an_exhausted_authority() {
    let (expected, mut reducer) = exhausted_reducer();
    let before = reducer.state().clone();
    let revision = reducer.revision();
    let start = envelope(
        &expected,
        CODEX_VERSION,
        FIRST_SEQUENCE,
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
    );

    assert_eq!(reducer.apply(start), Err(HookViolation::AuthorityExhausted));
    assert_eq!(reducer.state(), &before);
    assert_eq!(reducer.revision(), revision);
}

#[test]
fn invalid_event_cannot_fake_revocation_when_revision_is_exhausted() {
    let (expected, mut reducer) = exhausted_reducer();
    let before = reducer.state().clone();
    let gap = envelope(
        &expected,
        CODEX_VERSION,
        EventSequence::ZERO.0,
        EventKind::UserPromptSubmit,
    );

    assert_eq!(reducer.apply(gap), Err(HookViolation::AuthorityExhausted));
    assert_eq!(reducer.state(), &before);
    assert!(!reducer.state().revoked);
}

#[test]
fn process_exit_cannot_mutate_an_exhausted_authority() {
    let (_, mut reducer) = exhausted_reducer();
    let before = reducer.state().clone();
    let revision = reducer.revision();

    assert_eq!(
        reducer.process_exit(),
        ProcessExitDisposition::AuthorityExhausted
    );
    assert_eq!(reducer.state(), &before);
    assert_eq!(reducer.revision(), revision);
}
