use super::fixture::*;
use crate::{EventKind, SessionEndReason, SessionStartSource};
use yolo_agent_contract::AgentState;

#[test]
fn lifecycle_events_are_projected_and_replays_are_idempotent() {
    let fixture = Fixture::new();
    let started = fixture.apply(&fixture.start()).unwrap();
    assert_eq!(started.binding.state, AgentState::WaitingForInput);
    assert_eq!(started.revision, 1);
    assert_eq!(
        fixture.store.load_in_namespace(&fixture.namespace).unwrap(),
        Some(started.clone())
    );

    let running = fixture
        .apply(&fixture.request(2, EventKind::UserPromptSubmit, SESSION, &fixture.transcript))
        .unwrap();
    assert_eq!(running.binding.state, AgentState::Running);

    let ended_request = fixture.request(3, EventKind::TurnEnded, SESSION, &fixture.transcript);
    let ended = fixture.apply(&ended_request).unwrap();
    assert_eq!(ended.binding.state, AgentState::WaitingForInput);
    assert_eq!(ended.revision, 3);

    let replay = fixture.apply(&ended_request).unwrap();
    assert_eq!(replay, ended);
    assert_eq!(replay.revision, 3);
}

#[test]
fn binding_publication_is_compare_exchanged_and_replayable() {
    let fixture = Fixture::new();
    let started = fixture.apply(&fixture.start()).unwrap();
    for _ in 0..2 {
        fixture
            .store
            .publish_binding_in_namespace(&fixture.namespace, &started, || Ok(()))
            .unwrap();
    }
    let binding: yolo_agent_contract::AgentBinding = fixture
        .namespace
        .read_json(fixture.store.binding_path())
        .unwrap();
    assert_eq!(binding, started.recovery_binding(true));
    assert!(started.authorizes_resume(&binding));
}

#[test]
fn clear_replaces_the_session_and_transcript() {
    let fixture = Fixture::new();
    fixture.apply(&fixture.start()).unwrap();
    fixture
        .apply(&fixture.request(
            2,
            EventKind::SessionEnd {
                reason: SessionEndReason::Clear,
            },
            SESSION,
            &fixture.transcript,
        ))
        .unwrap();
    let next = fixture.cwd.join(NEXT_TRANSCRIPT);
    std::fs::write(&next, b"{\"sessionId\":\"next\"}\n").unwrap();
    let started = fixture
        .apply(&fixture.request(
            3,
            EventKind::SessionStart {
                source: SessionStartSource::Clear,
            },
            NEXT_SESSION,
            &next,
        ))
        .unwrap();
    assert_eq!(started.provider_session_id, NEXT_SESSION);
    assert_eq!(started.binding.provider_session_id, NEXT_SESSION);
    assert_eq!(started.binding.evidence_path, next.to_string_lossy());
    assert_eq!(started.binding.state, AgentState::WaitingForInput);
    assert_eq!(started.revision, 3);
}
