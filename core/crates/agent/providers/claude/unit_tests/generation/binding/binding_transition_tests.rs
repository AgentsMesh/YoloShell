use super::super::{GenerationEvent, InvocationMode, SessionEndReason, SessionStartSource};
use super::support::*;
use crate::agent::AgentState;

#[test]
fn binding_process_identity_and_state_must_match_the_authority() {
    let fixture = Fixture::new();
    let session_id = fixture.begin.provider_session_id.clone();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let authority = fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &fixture.start(&session_id, SessionStartSource::Fresh),
        )
        .unwrap()
        .0;
    let mut candidate = binding(&authority);
    assert!(authority.matches_binding(&candidate));
    candidate.process_start_time = Some(PROCESS_START + 1);
    assert!(!authority.matches_binding(&candidate));
    candidate.process_start_time = Some(PROCESS_START);
    candidate.state = AgentState::WaitingForInput;
    assert!(!authority.matches_binding(&candidate));
}

#[test]
fn resume_state_cwd_and_restart_transitions_are_generation_scoped() {
    let mut fixture = Fixture::new();
    fixture.begin.mode = InvocationMode::Resume;
    let resumed_id = fixture.begin.provider_session_id.clone();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let resumed = fixture.start(&resumed_id, SessionStartSource::Resume);
    let (mut authority, _) = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &resumed)
        .unwrap();
    assert_eq!(authority.state, AgentState::Running);
    let waiting = GenerationEvent::StateChanged {
        event_id: uuid::Uuid::new_v4(),
        provider_session_id: resumed_id.clone(),
        state: AgentState::WaitingForInput,
    };
    (authority, _) = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &waiting)
        .unwrap();
    assert_eq!(authority.state, AgentState::WaitingForInput);

    let moved_cwd = fixture.begin.cwd.join("nested");
    let cwd_event = GenerationEvent::CwdChanged {
        event_id: uuid::Uuid::new_v4(),
        provider_session_id: resumed_id.clone(),
        cwd: moved_cwd.clone(),
        transcript_path: fixture.transcript(&resumed_id),
    };
    (authority, _) = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &cwd_event)
        .unwrap();
    assert_eq!(authority.cwd, moved_cwd);
    let (recovering, _) = fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &end_with_reason(&resumed_id, SessionEndReason::Resume),
        )
        .unwrap();
    assert_eq!(recovering.state, AgentState::Unknown);
    let restarted_id = session_id();
    let restarted = fixture.start(&restarted_id, SessionStartSource::Resume);
    let (authority, _) = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &restarted)
        .unwrap();
    assert_eq!(authority.provider_session_id, restarted_id);
    assert_eq!(authority.generation, 2);
    assert_eq!(authority.state, AgentState::Running);
}
