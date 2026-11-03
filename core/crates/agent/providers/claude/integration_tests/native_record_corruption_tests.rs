use super::{NativeGenerations, fixture::Fixture};
use crate::agent::{AgentRecoveryAvailability, AgentState};
use crate::claude_hooks::{ClaudeLifecycleEvent, ClaudeSessionEndReason};

const NEXT_EVENT: u64 = crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE + 1;
const NEGATIVE_TIMESTAMP: i64 = -1;

#[test]
fn native_record_rejects_single_field_lifecycle_and_timestamp_corruption() {
    let fixture = Fixture::new();
    fixture.apply(&fixture.start).unwrap();
    let original: NativeGenerations = fixture.namespace.read_json(fixture.store.path()).unwrap();
    let mutations: &[fn(&mut super::ClaudeNativeGeneration)] = &[
        |record| record.final_session_end = true,
        |record| record.binding.detected_at_ms = NEGATIVE_TIMESTAMP,
        |record| record.binding.provider_session_id = uuid::Uuid::new_v4().to_string(),
        |record| record.revoked = true,
        |record| record.binding.recovery_availability = AgentRecoveryAvailability::Unavailable,
    ];
    for mutate in mutations {
        let mut document = original.clone();
        mutate(&mut document.invocations[0]);
        std::fs::write(fixture.store.path(), serde_json::to_vec(&document).unwrap()).unwrap();
        assert!(fixture.store.load_in_namespace(&fixture.namespace).is_err());
    }
}

#[test]
fn matching_binding_and_activity_cannot_override_the_last_hook_state() {
    let fixture = Fixture::new();
    fixture.apply(&fixture.start).unwrap();
    let mut document: NativeGenerations =
        fixture.namespace.read_json(fixture.store.path()).unwrap();
    document.invocations[0].activity = AgentState::Running;
    document.invocations[0].binding.state = AgentState::Running;
    std::fs::write(fixture.store.path(), serde_json::to_vec(&document).unwrap()).unwrap();
    assert!(fixture.store.load_in_namespace(&fixture.namespace).is_err());
}

#[test]
fn terminal_hook_proof_cannot_be_cleared_without_a_subsequent_start() {
    let fixture = Fixture::new();
    fixture.apply(&fixture.start).unwrap();
    let mut end = fixture.state_event(NEXT_EVENT);
    end.event = ClaudeLifecycleEvent::SessionEnd {
        event_id: uuid::Uuid::new_v4(),
        provider_session_id: fixture.start.event.provider_session_id().to_owned(),
        reason: ClaudeSessionEndReason::Other,
    };
    let ended = fixture.apply(&end).unwrap();
    assert!(ended.final_session_end);
    let mut document: NativeGenerations =
        fixture.namespace.read_json(fixture.store.path()).unwrap();
    document.invocations[0].final_session_end = false;
    std::fs::write(fixture.store.path(), serde_json::to_vec(&document).unwrap()).unwrap();
    assert!(fixture.store.load_in_namespace(&fixture.namespace).is_err());
}
