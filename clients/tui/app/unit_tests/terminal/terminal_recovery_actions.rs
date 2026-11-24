use super::fixtures;
use crate::{Mode, State, actions::handle, constants::QUEUE_CAPACITY, network::Job};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{
    RuntimeStatus, TerminalAction, TerminalRecoveryHint, TerminalSummary,
};

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn setup(hint: Option<TerminalRecoveryHint>) -> (State, TerminalSummary) {
    let mut fields = fixtures::session().fields().clone();
    fields.runtime.as_mut().unwrap().status = RuntimeStatus::Exited;
    fields.recovery = hint;
    let terminal = TerminalSummary::new(fields).unwrap();
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.status = yolo_hub_terminal_protocol::PublicError::RecreateRequired.to_string();
    state.replace_terminals(vec![terminal.clone()]);
    crate::tabs::terminal_picker::open(&mut state);
    state.inventory.history = true;
    state.reconcile_selection();
    (state, terminal)
}

#[test]
fn shell_recreation_requires_confirmation_and_keeps_the_original_runtime_selection() {
    let (mut state, terminal) = setup(Some(TerminalRecoveryHint::Recreate));
    let hub = terminal.fields().address.hub_id;
    let target = terminal.fields().runtime.unwrap().target;
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    handle(&mut state, &jobs, hub, key(KeyCode::Char('R')));
    assert_eq!(state.mode, Mode::ConfirmRecreate);
    assert!(state.status.is_empty());
    assert!(incoming.try_recv().is_err());
    state.replace_terminals(vec![fixtures::session()]);
    handle(&mut state, &jobs, hub, key(KeyCode::Enter));
    let Job::Call {
        command: Command::Terminal { command },
        ..
    } = incoming.try_recv().unwrap()
    else {
        panic!()
    };
    assert_eq!(command.action, TerminalAction::Recreate {});
    assert_eq!(command.selection.address, target.address);
    assert_eq!(
        command.selection.expected_runtime,
        Some(target.runtime_epoch)
    );
    assert!(incoming.try_recv().is_err());
}

#[test]
fn cancelled_or_unavailable_recovery_never_sends_a_mutation() {
    for hint in [
        None,
        Some(TerminalRecoveryHint::Unavailable),
        Some(TerminalRecoveryHint::Recovering),
        Some(TerminalRecoveryHint::Recreate),
    ] {
        let (mut state, terminal) = setup(hint);
        let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
        let hub = terminal.fields().address.hub_id;
        handle(&mut state, &jobs, hub, key(KeyCode::Char('R')));
        assert!(incoming.try_recv().is_err());
        handle(&mut state, &jobs, hub, key(KeyCode::Esc));
        assert!(incoming.try_recv().is_err());
        assert!(state.confirmation.is_none());
    }
}

#[test]
fn available_provider_history_uses_resume_without_recreating_the_shell() {
    let (mut state, terminal) = setup(Some(TerminalRecoveryHint::Resume));
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    handle(
        &mut state,
        &jobs,
        terminal.fields().address.hub_id,
        key(KeyCode::Char('R')),
    );
    let Job::Call {
        command: Command::Terminal { command },
        ..
    } = incoming.try_recv().unwrap()
    else {
        panic!()
    };
    assert_eq!(command.action, TerminalAction::Resume {});
    assert_ne!(state.mode, Mode::ConfirmRecreate);
}
