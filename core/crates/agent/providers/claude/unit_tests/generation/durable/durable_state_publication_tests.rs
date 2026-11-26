use super::super::{GenerationApplyDisposition, GenerationEvent, SessionStartSource};
use super::support::*;
use crate::agent::claude::generation::adversarial_test_support::prepare_durable_state_for_test;
use crate::agent::{AgentBindingStore, AgentRecoveryAvailability, AgentState};

#[test]
fn initial_session_start_creates_and_commits_the_binding() {
    let fixture = Fixture::new();
    let (authority, event, transcript) = begin_and_start(&fixture);
    let (applied, disposition, reactivated) = fixture
        .store
        .transition_foreground_durable_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &authority,
            next_sequence(&authority),
            &event,
            Some(transcript),
        )
        .unwrap();
    assert_eq!(disposition, GenerationApplyDisposition::Applied);
    assert!(!reactivated);

    let (committed, binding) = fixture
        .store
        .publish_binding_state_in_namespace(
            &fixture.lease,
            &fixture.binding_path(),
            fixture.authentication(),
            &applied,
            false,
            |_| Ok(()),
        )
        .unwrap();
    assert!(committed.matches_binding(&binding));
    assert_eq!(binding.state, AgentState::Running);
    assert_eq!(
        AgentBindingStore::new(fixture.binding_path())
            .read_raw_in_namespace(&fixture.lease)
            .unwrap(),
        Some(binding)
    );
}

#[test]
fn pending_unknown_retry_publishes_the_exact_staged_target() {
    let fixture = published_fixture();
    let previous_binding = binding_at(&fixture);
    let pending =
        prepare_durable_state_for_test(&fixture.binding_path(), AgentState::Unknown).unwrap();
    assert_eq!(binding_at(&fixture), previous_binding);

    let (_, target) = fixture
        .store
        .publish_binding_state_in_namespace(
            &fixture.lease,
            &fixture.binding_path(),
            fixture.authentication(),
            &pending,
            false,
            |_| Ok(()),
        )
        .unwrap();
    assert_eq!(binding_at(&fixture), target);
    assert_eq!(target.state, AgentState::Unknown);
    assert_eq!(
        target.recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
    assert_eq!(target.evidence_path, previous_binding.evidence_path);
    assert!(authority(&fixture).binding_revision.is_none());

    let timestamp = target.updated_at_ms;
    let current = authority(&fixture);
    let (_, replayed) = fixture
        .store
        .publish_binding_state_in_namespace(
            &fixture.lease,
            &fixture.binding_path(),
            fixture.authentication(),
            &current,
            false,
            |_| Ok(()),
        )
        .unwrap();
    assert_eq!(replayed, target);
    assert_eq!(replayed.updated_at_ms, timestamp);
}

#[test]
fn a_binding_changed_after_snapshot_fails_closed() {
    let fixture = published_fixture();
    let authority_before = authority(&fixture);
    let before = binding_at(&fixture);
    let mut foreign = before.clone();
    foreign.provider_session_id = session_id();
    fixture
        .lease
        .write_json(&fixture.binding_path(), &foreign)
        .unwrap();

    assert!(
        fixture
            .store
            .publish_binding_state_in_namespace(
                &fixture.lease,
                &fixture.binding_path(),
                fixture.authentication(),
                &authority_before,
                true,
                |_| Ok(()),
            )
            .is_err()
    );
    assert_eq!(binding_at(&fixture), foreign);
    assert_eq!(authority(&fixture), authority_before);
}

fn begin_and_start(
    fixture: &Fixture,
) -> (
    super::super::ClaudeGenerationAuthority,
    GenerationEvent,
    crate::paths::identity::ObjectIdentity,
) {
    let authority = fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let event = fixture.start(&fixture.begin.provider_session_id, SessionStartSource::Fresh);
    let GenerationEvent::SessionStart {
        transcript_path, ..
    } = &event
    else {
        unreachable!();
    };
    let transcript = crate::paths::identity::ObjectIdentity::from_file(
        &std::fs::File::open(transcript_path).unwrap(),
    )
    .unwrap()
    .unwrap();
    (authority, event, transcript)
}

pub(super) fn published_fixture() -> Fixture {
    let fixture = Fixture::new();
    let (authority, event, transcript) = begin_and_start(&fixture);
    let (applied, _, _) = fixture
        .store
        .transition_foreground_durable_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &authority,
            next_sequence(&authority),
            &event,
            Some(transcript),
        )
        .unwrap();
    fixture
        .store
        .publish_binding_state_in_namespace(
            &fixture.lease,
            &fixture.binding_path(),
            fixture.authentication(),
            &applied,
            false,
            |_| Ok(()),
        )
        .unwrap();
    fixture
}

fn next_sequence(authority: &super::super::ClaudeGenerationAuthority) -> u64 {
    authority
        .last_event_sequence
        .checked_add(super::super::constants::REVISION_INCREMENT)
        .unwrap()
}

fn authority(fixture: &Fixture) -> super::super::ClaudeGenerationAuthority {
    fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap()
}

fn binding_at(fixture: &Fixture) -> crate::agent::AgentBinding {
    AgentBindingStore::new(fixture.binding_path())
        .read_raw_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap()
}
