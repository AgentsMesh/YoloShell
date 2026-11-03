use super::{chat_support::*, drawing_support, interrupt_support};
use crate::{
    chat, events,
    network::{Job, Update},
};
use crossterm::event::{KeyCode, KeyModifiers};
use yolo_hub_agent_protocol::{CommandOutcome, FailureCode};
use yolo_hub_catalog_client::Catalog;
use yolo_hub_client_channels::CatalogEvent;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{LeaseGrant, SecretCapability};
use yolo_tui_chat::InputReadiness;

const GENERATION_STEP: u64 = 1;
const SEND_HINT: &str = "Enter append & send";

#[test]
fn rejected_writer_precondition_accepts_the_same_agent_cursor_after_a_fresh_read_and_grant() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    let snapshot = chat::agent(&state).unwrap().clone();
    let terminals = state.terminals.values().cloned().collect();
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let first = sent(&mut incoming);
    let reply = interrupt_support::reply(
        &first,
        CommandOutcome::Failed {
            code: FailureCode::PreconditionsChanged,
        },
    );
    interrupt_support::finish(&mut state, &jobs, first.clone(), reply);
    assert_eq!(
        chat::input_readiness(&state),
        InputReadiness::AwaitingObservation
    );
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    assert!(incoming.try_recv().is_err());
    chat::pump(&mut state, &jobs);
    let Job::Refresh { request_id } = incoming.try_recv().unwrap() else {
        panic!()
    };
    assert!(!state.inventory.current);
    assert!(incoming.try_recv().is_err());
    events::apply(
        &mut state,
        &jobs,
        Update::Catalog(CatalogEvent {
            request_id,
            result: Ok(Catalog {
                terminals,
                agents: Some(vec![snapshot.clone()]),
                hubs: None,
            }),
        }),
    );
    let address = snapshot.fields().terminal.address;
    let mut fields = *state.writers[&address].grant.fields();
    fields.generation += GENERATION_STEP;
    fields.capability = SecretCapability::generate();
    let fresh = LeaseGrant::new(fields).unwrap();
    state.record_grant(fresh, std::time::Instant::now());
    assert_eq!(chat::input_readiness(&state), InputReadiness::Ready);
    assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    assert!(incoming.try_recv().is_err());
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let manual = sent(&mut incoming);
    let Command::AgentControl { request, grant } = manual.command else {
        panic!()
    };
    assert_eq!(request.fields().expected_cursor, snapshot.cursor());
    assert_eq!(grant, fresh);
    assert_ne!(
        request.fields().request_id,
        interrupt_support::request(&first).fields().request_id
    );
}

#[test]
fn first_chat_view_without_input_control_never_renders_the_send_key_hint() {
    let (mut state, jobs, _) = ready();
    toggle(&mut state, &jobs);
    state.writers.clear();
    assert!(chat::current(&state).unwrap().composer.phase().is_none());
    assert_eq!(chat::input_readiness(&state), InputReadiness::Unavailable);
    assert!(
        !drawing_support::render(&state, WIDTH, HEIGHT)
            .0
            .contains(SEND_HINT)
    );
}
