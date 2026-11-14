use super::{chat_projection, chat_support::*};
use crate::{
    Mode, chat, client_capability_support, hub_actions,
    network::{Job, Update},
};
use crossterm::event::{KeyCode, KeyModifiers};
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_rpc_protocol::Method;
use yolo_tui_chat::{InputReadiness, QueryError};

#[test]
fn conversation_loss_cancels_reads_and_preserves_an_editable_local_draft() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    let pending = chat_projection::operation(&mut state, &jobs, &mut incoming);
    let descriptor = chat_projection::descriptor(&state);
    let terminal = state.focused_terminal();
    client_capability_support::install_without(&mut state, &[Method::ReadConversation]);
    chat::pump(&mut state, &jobs);
    chat::completed(
        &mut state,
        pending,
        Ok(chat::network::Reading::Resolved(descriptor)),
    );
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    assert!(incoming.try_recv().is_err());
    let chat = chat::current(&state).unwrap();
    assert_eq!(chat.composer.text(), PROMPT);
    assert!(!chat.retired());
    assert!(chat.pending().is_none());
    assert!(chat.descriptor().is_none());
    assert_eq!(state.focused_terminal(), terminal);
    assert_eq!(chat::input_readiness(&state), InputReadiness::Unavailable);
    client_capability_support::install(&mut state);
    chat::pump(&mut state, &jobs);
    assert!(matches!(incoming.try_recv().unwrap(), Job::Chat(_)));
}

#[test]
fn loss_of_agent_control_keeps_chat_readable_without_send_or_interrupt_requests() {
    let (mut state, jobs, mut incoming) = ready();
    client_capability_support::install_without(&mut state, &[Method::AgentControl]);
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    chat::pump(&mut state, &jobs);
    assert!(matches!(incoming.try_recv().unwrap(), Job::Chat(_)));
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    assert!(incoming.try_recv().is_err());
    assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    state.mode = Mode::Inventory;
    crate::interrupt_actions::start(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    assert_eq!(
        state.status,
        crate::client_capabilities::AGENT_CONTROL_UNAVAILABLE
    );
    assert!(!crate::interrupt_actions::status(&state).contains(crate::interrupt_constants::HINT));
}

#[test]
fn missing_agent_observation_cannot_open_chat_even_with_a_retained_snapshot() {
    let (mut state, jobs, mut incoming) = ready();
    client_capability_support::install_without(&mut state, &[Method::ListAgents]);
    toggle(&mut state, &jobs);
    chat::pump(&mut state, &jobs);
    assert_eq!(state.mode, Mode::Terminal);
    assert!(state.chats.entries.is_empty());
    assert!(incoming.try_recv().is_err());
}

#[test]
fn unsupported_reader_stops_polling_until_a_new_view_epoch() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    let query = chat_projection::operation(&mut state, &jobs, &mut incoming);
    chat::completed(&mut state, query, Err(QueryError::Unsupported));
    let chat_key = chat::Key::current(&state);
    state.chats.entries.get_mut(&chat_key).unwrap().next = std::time::Instant::now();
    chat::pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    key(&mut state, &jobs, KeyCode::Esc, KeyModifiers::NONE);
    toggle(&mut state, &jobs);
    chat::pump(&mut state, &jobs);
    assert!(matches!(incoming.try_recv().unwrap(), Job::Chat(_)));
}

#[test]
fn management_loss_closes_its_pending_view_and_ignores_late_result() {
    let mut state = crate::client_capability_support::state();
    client_capability_support::install(&mut state);
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let hub = yolo_hub_identity::HubId::new();
    hub_actions::open(&mut state, &jobs, hub);
    let Job::Management(operation) = incoming.try_recv().unwrap() else {
        panic!()
    };
    client_capability_support::install_without(&mut state, &[Method::Configure]);
    assert!(!state.client_capability_enabled(ClientCapability::HubManagement));
    assert!(state.hub_manager.is_none());
    assert_eq!(state.mode, Mode::Inventory);
    crate::events::apply(
        &mut state,
        &jobs,
        Update::Management {
            operation,
            result: Err(yolo_hub_management_client::ManagementError::Unavailable),
        },
    );
    hub_actions::open(&mut state, &jobs, hub);
    assert!(state.hub_manager.is_none());
    assert!(incoming.try_recv().is_err());
    assert_eq!(
        state.status,
        crate::client_capabilities::HUB_MANAGEMENT_UNAVAILABLE
    );
}
