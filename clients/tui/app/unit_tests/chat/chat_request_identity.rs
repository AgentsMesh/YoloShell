use super::{chat_support::*, interrupt_support};
use crate::chat;
use crossterm::event::{KeyCode, KeyModifiers};
use yolo_hub_agent_protocol::CommandOutcome;
use yolo_hub_rpc_protocol::Command;

#[test]
fn same_request_id_with_changed_writer_or_local_deadline_cannot_acknowledge_submission() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let request = sent(&mut incoming);
    let reply = interrupt_support::reply(&request, CommandOutcome::Delivered {});
    let mut changed = request.clone();
    let Command::AgentControl {
        request: agent_request,
        grant,
    } = &request.command
    else {
        panic!()
    };
    let mut fields = *grant.fields();
    fields.principal.client_id = uuid::Uuid::new_v4();
    changed.command = Command::AgentControl {
        request: agent_request.clone(),
        grant: yolo_hub_terminal_protocol::LeaseGrant::new(fields).unwrap(),
    };
    assert!(chat::control(
        &mut state,
        &changed,
        &Ok(reply.clone()),
        false
    ));
    assert!(state.control_generation.is_some());
    let id = chat::Key::current(&state);
    let scope = &state.chats.entries[&id].submission.as_ref().unwrap()._scope;
    let mut changed = request.clone();
    changed.validity = Some(scope.validity_until(
        request.validity.as_ref().unwrap().deadline() + crate::constants::RPC_TIMEOUT,
    ));
    assert!(chat::control(
        &mut state,
        &changed,
        &Ok(reply.clone()),
        false
    ));
    assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    assert!(chat::control(&mut state, &request, &Ok(reply), true));
    assert!(chat::not_sent(&mut state, &request));
    assert_eq!(
        chat::current(&state).unwrap().composer.phase(),
        Some(yolo_tui_chat::SendPhase::Delivered)
    );
}
