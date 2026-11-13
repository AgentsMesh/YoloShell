use super::{chat_support::*, drawing_support, interrupt_support};
use crate::{
    Mode,
    chat::{self, network::Reading},
    network::Job,
};
use crossterm::event::{KeyCode, KeyModifiers};
use yolo_conversation_protocol::*;
use yolo_hub_agent_protocol::{AgentSnapshot, ObservationCursor};

fn replace_execution(state: &mut crate::State) {
    interrupt_support::mutate_agent(state, |fields| {
        let mut target = fields.cursor.target();
        target.execution_id = yolo_hub_identity::ExecutionId::new();
        fields.cursor = ObservationCursor::new(
            target,
            uuid::Uuid::new_v4(),
            yolo_hub_agent_protocol::FIRST_OBSERVATION_SEQUENCE,
        )
        .unwrap();
    });
}

#[test]
fn temporary_catalog_refresh_does_not_retire_a_retained_chat_draft() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Esc, KeyModifiers::NONE);
    state.inventory.current = false;
    toggle(&mut state, &jobs);
    assert_eq!(state.mode, Mode::Chat);
    assert!(!chat::current(&state).unwrap().retired());
    assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    chat::pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    assert!(incoming.try_recv().is_err());
    assert!(state.status.contains("Refreshing agent state"));
    state.inventory.current = true;
    chat::synchronize(&mut state);
    assert!(!chat::current(&state).unwrap().retired());
    assert_eq!(state.status, crate::constants::CONNECTED_LABEL);
}

#[test]
fn replaced_execution_cannot_complete_old_query_and_retained_draft_has_a_reachable_discard_path() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    chat::pump(&mut state, &jobs);
    let Job::Chat(operation) = incoming.try_recv().unwrap() else {
        panic!()
    };
    let before = chat::current(&state).unwrap().binding();
    let descriptor = ConversationDescriptor {
        address: ConversationAddress {
            hub_id: before.agent.address.hub_id,
            conversation_id: yolo_crypto::Sha256Digest::capture(&NATIVE_SESSION).unwrap(),
        },
        binding: before,
        provider: ConversationProvider::codex(),
        provider_session_id: NATIVE_SESSION.into(),
        source: ConversationSource::NativeTranscript,
    };
    key(&mut state, &jobs, KeyCode::Esc, KeyModifiers::NONE);
    replace_execution(&mut state);
    chat::completed(&mut state, operation, Ok(Reading::Resolved(descriptor)));
    assert!(chat::current(&state).unwrap().descriptor().is_none());
    assert!(chat::current(&state).unwrap().retired());
    chat::pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    toggle(&mut state, &jobs);
    assert_eq!(state.mode, Mode::Chat);
    assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    assert!(
        drawing_support::render(&state, WIDTH, HEIGHT)
            .0
            .contains("Previous agent draft")
    );
    paste(&mut state, &jobs, NEXT_PROMPT);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    assert!(incoming.try_recv().is_err());
    key(&mut state, &jobs, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(state.mode, Mode::Chat);
    let current = chat::current(&state).unwrap();
    assert_ne!(current.binding().agent, before.agent);
    assert!(current.composer.text().is_empty());
    assert!(!current.retired());
}

#[test]
fn exited_agent_draft_remains_visible_and_discard_returns_to_the_existing_shell() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Esc, KeyModifiers::NONE);
    let id = state.inventory.selected_agent.unwrap();
    let mut fields = state.inventory.agents[&id].fields().clone();
    fields.lifecycle = yolo_hub_agent_protocol::Lifecycle::Exited { exit_code: Some(0) };
    fields.activity = yolo_hub_agent_protocol::Activity::Idle;
    fields.capabilities = yolo_hub_agent_protocol::AgentCapabilities::UNSUPPORTED;
    state
        .inventory
        .agents
        .insert(id, AgentSnapshot::new(fields).unwrap());
    toggle(&mut state, &jobs);
    assert_eq!(state.mode, Mode::Chat);
    assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    key(&mut state, &jobs, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(state.mode, Mode::Terminal);
    assert!(chat::current(&state).is_none());
    assert!(state.focused_terminal().is_some());
    assert!(incoming.try_recv().is_err());
}
