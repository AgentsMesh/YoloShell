use super::fixture::Fixture;
use super::*;
use crate::claude_hooks::{ClaudeLifecycleEvent, ClaudeSessionEndReason, ClaudeSessionStartSource};

const SECOND_EVENT: u64 = 2;
const GAP_EVENT: u64 = 3;

#[test]
fn first_start_and_exact_replay_have_one_invocation() {
    let fixture = Fixture::new();
    let mut first = reducer::first(
        &fixture.start,
        fixture.worker,
        &fixture.boot_id,
        &fixture.evidence,
    )
    .unwrap();
    let expected = first.clone();
    assert!(!reducer::apply(&mut first, &fixture.start, &fixture.evidence).unwrap());
    assert_eq!(first, expected);
}

#[test]
fn non_start_and_non_initial_sequence_cannot_register() {
    let fixture = Fixture::new();
    assert!(
        reducer::first(
            &fixture.state_event(crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE),
            fixture.worker,
            &fixture.boot_id,
            &fixture.evidence
        )
        .is_err()
    );
    let mut request = fixture.start.clone();
    request.event_sequence = SECOND_EVENT;
    assert!(
        reducer::first(
            &request,
            fixture.worker,
            &fixture.boot_id,
            &fixture.evidence
        )
        .is_err()
    );
}

#[test]
fn sequence_gap_event_substitution_and_pid_reuse_fail_closed() {
    let fixture = Fixture::new();
    let first = reducer::first(
        &fixture.start,
        fixture.worker,
        &fixture.boot_id,
        &fixture.evidence,
    )
    .unwrap();
    let gap = fixture.state_event(GAP_EVENT);
    let conflict = fixture.state_event(crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE);
    let mut reused = fixture.state_event(SECOND_EVENT);
    reused.scope.agent.start_time += INCREMENT;
    for request in [gap, conflict, reused] {
        assert!(reducer::apply(&mut first.clone(), &request, &fixture.evidence).is_err());
    }
}

#[test]
fn same_process_compaction_does_not_create_an_invocation() {
    let fixture = Fixture::new();
    let mut first = reducer::first(
        &fixture.start,
        fixture.worker,
        &fixture.boot_id,
        &fixture.evidence,
    )
    .unwrap();
    let invocation = first.invocation_id;
    let mut compact = fixture.start.clone();
    compact.event_sequence = SECOND_EVENT;
    if let ClaudeLifecycleEvent::SessionStart {
        event_id, source, ..
    } = &mut compact.event
    {
        *event_id = Uuid::new_v4();
        *source = ClaudeSessionStartSource::Compact;
    }
    assert!(reducer::apply(&mut first, &compact, &fixture.evidence).unwrap());
    assert_eq!(first.invocation_id, invocation);
}

#[test]
fn missing_end_keeps_resume_evidence_but_not_live_activity() {
    let fixture = Fixture::new();
    let first = reducer::first(
        &fixture.start,
        fixture.worker,
        &fixture.boot_id,
        &fixture.evidence,
    )
    .unwrap();
    let recovered = first.recovery_binding(false);
    assert_eq!(recovered.state, AgentState::Unknown);
    assert_eq!(recovered.process_pid, Some(first.scope.agent.pid));
    assert_eq!(
        recovered.recovery_availability,
        crate::agent::AgentRecoveryAvailability::Available
    );
    assert!(first.authorizes_resume(&recovered));
}

#[test]
fn final_end_does_not_publish_process_exit_until_os_proof() {
    let fixture = Fixture::new();
    let mut first = reducer::first(
        &fixture.start,
        fixture.worker,
        &fixture.boot_id,
        &fixture.evidence,
    )
    .unwrap();
    let mut request = fixture.state_event(SECOND_EVENT);
    request.event = ClaudeLifecycleEvent::SessionEnd {
        event_id: Uuid::new_v4(),
        provider_session_id: fixture.start.event.provider_session_id().to_owned(),
        reason: ClaudeSessionEndReason::Other,
    };
    reducer::apply(&mut first, &request, &fixture.evidence).unwrap();
    assert!(first.final_session_end);
    assert_eq!(first.activity, AgentState::Unknown);
    assert!(first.verify_resume_evidence().unwrap());
}

#[test]
fn explicit_revocation_cannot_be_replayed_or_recovered() {
    let fixture = Fixture::new();
    let mut first = reducer::first(
        &fixture.start,
        fixture.worker,
        &fixture.boot_id,
        &fixture.evidence,
    )
    .unwrap();
    first.revoked = true;
    assert!(reducer::apply(&mut first, &fixture.start, &fixture.evidence).is_err());
    assert!(!first.verify_resume_evidence().unwrap());
    assert!(!first.authorizes_resume(&first.binding));
}
