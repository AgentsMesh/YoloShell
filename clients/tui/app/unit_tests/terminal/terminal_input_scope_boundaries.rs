use super::{
    query_support::Fixture, terminal_input_navigation_support::PREFIX,
    terminal_input_ordering::queued,
};
use crate::{Mode, terminal_actions, terminal_input_authority, terminal_input_scope::Scope};
use crossterm::event::KeyCode;
use yolo_hub_client_channels::ControlError;
use yolo_hub_terminal_protocol::TerminalSummary;

#[test]
fn tab_manager_retires_queued_keyboard_input_without_revoking_the_terminal_writer() {
    let mut fixture = Fixture::new();
    let input = queued(&mut fixture, PREFIX);

    terminal_actions::prefix(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        KeyCode::Char(crate::tabs::MANAGER_KEY),
    );
    fixture.state.sync_query_authority();

    assert_eq!(fixture.state.mode, Mode::Terminal);
    assert!(fixture.state.tab_manager.is_some());
    assert!(fixture.state.inputs.scope.is_none());
    assert!(!input.validity.as_ref().unwrap().is_current());
    fixture.finish(input, Err(ControlError::Invalidated), true);
    assert!(!terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        fixture.grant
    );
    assert!(fixture.state.inputs.pending.is_empty());
    fixture.no_jobs();
}

#[test]
fn a_view_scope_cannot_authorize_a_new_runtime_at_the_same_terminal_address() {
    let mut fixture = Fixture::new();
    let input = queued(&mut fixture, PREFIX);
    let scope = Scope::new(
        &fixture.state,
        fixture.state.control_generation.unwrap(),
        fixture.grant,
    );
    assert!(scope.current(&fixture.state));
    let mut terminal = fixture.state.terminals[&fixture.target.address]
        .fields()
        .clone();
    terminal.runtime.as_mut().unwrap().target.runtime_epoch =
        yolo_hub_identity::RuntimeEpoch::new();

    fixture
        .state
        .replace_terminals(vec![TerminalSummary::new(terminal).unwrap()]);

    // Location and connection are unchanged; runtime identity alone must fence
    // an independently retained view value before another input is admitted.
    assert!(scope.same_view(&fixture.state));
    assert!(!scope.current(&fixture.state));
    assert!(!input.validity.as_ref().unwrap().is_current());
    assert!(fixture.state.inputs.scope.is_none());
    assert!(fixture.state.writers.is_empty());
    fixture.finish(input, Err(ControlError::Invalidated), true);
    assert!(fixture.state.inputs.pending.is_empty());
    fixture.no_jobs();
}
