use super::super::{
    GenerationApplyDisposition, GenerationEvent, InvocationMode, SessionEndReason,
    SessionStartSource,
};
use super::support::*;
use crate::agent::AgentState;

#[test]
fn clear_compact_and_every_late_event_preserve_the_current_generation() {
    let fixture = Fixture::new();
    let first_id = fixture.begin.provider_session_id.clone();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let (mut record, _) = fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &fixture.start(&first_id, SessionStartSource::Fresh),
        )
        .unwrap();
    assert!(record.matches_binding(&binding(&record)));
    let second_id = session_id();
    (record, _) = fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &end_with_reason(&first_id, SessionEndReason::Clear),
        )
        .unwrap();
    assert_eq!(record.state, AgentState::Unknown);
    (record, _) = fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &fixture.start(&second_id, SessionStartSource::Clear),
        )
        .unwrap();
    assert_eq!(record.generation, 2);
    let active_revision = record.revision;
    let late = end(&first_id);
    let (mut stale, disposition) = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &late)
        .unwrap();
    assert_eq!(disposition, GenerationApplyDisposition::Stale);
    assert_eq!(stale.provider_session_id, second_id);
    assert_eq!(stale.state, AgentState::Running);
    assert_eq!(stale.revision, active_revision);
    let late_events = [
        GenerationEvent::StateChanged {
            event_id: uuid::Uuid::new_v4(),
            provider_session_id: first_id.clone(),
            state: AgentState::WaitingForInput,
        },
        GenerationEvent::CwdChanged {
            event_id: uuid::Uuid::new_v4(),
            provider_session_id: first_id.clone(),
            cwd: fixture.begin.cwd.join("stale"),
            transcript_path: fixture.transcript(&first_id),
        },
    ];
    for event in late_events {
        let (next, disposition) = fixture
            .store
            .apply_in_namespace(&fixture.lease, fixture.authentication(), &event)
            .unwrap();
        stale = next;
        assert_eq!(disposition, GenerationApplyDisposition::Stale);
        assert_eq!(stale.provider_session_id, second_id);
        assert_eq!(stale.state, AgentState::Running);
    }

    let compact = fixture.start(&second_id, SessionStartSource::Compact);
    let (compacted, _) = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &compact)
        .unwrap();
    let (replayed, disposition) = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &compact)
        .unwrap();
    assert_eq!(disposition, GenerationApplyDisposition::Replay);
    assert_eq!(compacted, replayed);
}

#[test]
fn active_sessions_can_resume_a_previous_id_as_a_new_generation() {
    let fixture = Fixture::new();
    let first_id = fixture.begin.provider_session_id.clone();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &fixture.start(&first_id, SessionStartSource::Fresh),
        )
        .unwrap();
    let second_id = session_id();
    let clear = fixture.start(&second_id, SessionStartSource::Clear);
    let (active, disposition) = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &clear)
        .unwrap();
    assert_eq!(disposition, GenerationApplyDisposition::Applied);
    assert_eq!(active.provider_session_id, second_id);
    let resumed = fixture.start(&first_id, SessionStartSource::Resume);
    let (reactivated, disposition) = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &resumed)
        .unwrap();
    assert_eq!(disposition, GenerationApplyDisposition::Applied);
    assert_eq!(reactivated.provider_session_id, first_id);
    assert_eq!(reactivated.generation, active.generation + 1);
}

#[test]
fn picker_and_fork_invocations_bind_the_hook_observed_session() {
    for source in [SessionStartSource::Resume, SessionStartSource::Fork] {
        let mut fixture = Fixture::new();
        fixture.begin.mode = InvocationMode::Resume;
        fixture.begin.discover_session = true;
        let placeholder = fixture.begin.provider_session_id.clone();
        fixture
            .store
            .begin_in_namespace(&fixture.lease, &fixture.begin)
            .unwrap();
        let discovered = session_id();
        let (authority, disposition) = fixture
            .store
            .apply_in_namespace(
                &fixture.lease,
                fixture.authentication(),
                &fixture.start(&discovered, source),
            )
            .unwrap();
        assert_eq!(disposition, GenerationApplyDisposition::Applied);
        assert_ne!(authority.provider_session_id, placeholder);
        assert_eq!(authority.provider_session_id, discovered);
        assert!(!authority.discover_session);
        assert_eq!(authority.generation, 1);
    }
}
