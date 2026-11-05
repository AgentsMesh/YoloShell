use super::{fixtures, panes};
use crate::{Mode, State, tabs};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::{OutputCursor, RuntimeStatus, TerminalEvent, TerminalSummary};
use yolo_tui_tab::{LocalTabView, TabViewMode};

const OTHER_TAB: &str = "Two";

fn ended(session: &TerminalSummary) -> TerminalSummary {
    let mut fields = session.fields().clone();
    fields.runtime.as_mut().unwrap().status = RuntimeStatus::Exited;
    TerminalSummary::new(fields).unwrap()
}

fn bind_as_terminal_tab(state: &mut State, address: TerminalAddress) {
    state.bind_focused(Some(address)).unwrap();
    state.tabs.update_active(LocalTabView {
        layout: state.tab_layout().clone(),
        view: TabViewMode::Terminal,
    });
}

fn state_with(sessions: &[TerminalSummary]) -> State {
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(Uuid::new_v4());
    state.replace_terminals(sessions.to_vec());
    state
}

#[test]
fn exited_only_tab_retires_its_pane_and_keeps_the_record_for_history() {
    let session = fixtures::session();
    let address = session.fields().address;
    let mut state = state_with(std::slice::from_ref(&session));
    bind_as_terminal_tab(&mut state, address);
    state.mode = Mode::Terminal;

    state.replace_terminals(vec![ended(&session)]);

    assert_eq!(state.focused_terminal(), None);
    assert_eq!(state.tabs.entries().len(), 1);
    assert!(
        state
            .tabs
            .active()
            .view
            .layout
            .panes()
            .iter()
            .all(|pane| pane.terminal.is_none())
    );
    assert_eq!(state.mode, Mode::Inventory);
    assert!(state.sidebar_visible);
    assert!(state.terminals.contains_key(&address));
    assert!(
        state
            .inventory
            .visible_terminals("", &state.terminals)
            .is_empty()
    );
    state.inventory.history = true;
    assert_eq!(
        state.inventory.visible_terminals("", &state.terminals),
        vec![address]
    );
}

#[test]
fn exited_tab_does_not_remove_its_live_sibling() {
    let first = fixtures::session();
    let second = fixtures::session();
    let second_address = second.fields().address;
    let mut state = state_with(&[first.clone(), second.clone()]);
    bind_as_terminal_tab(&mut state, first.fields().address);
    state.tabs.create(OTHER_TAB).unwrap();
    bind_as_terminal_tab(&mut state, second_address);
    state.mode = Mode::Terminal;

    state.replace_terminals(vec![ended(&first), second]);

    assert_eq!(state.tabs.entries().len(), 1);
    assert_eq!(state.tabs.active().name, OTHER_TAB);
    assert_eq!(state.focused_terminal(), Some(second_address));
    assert_eq!(state.mode, Mode::Terminal);
    assert!(!state.sidebar_visible);
}

#[test]
fn exited_pane_leaves_the_rest_of_its_tab_interactive() {
    let (mut state, first, second) = panes::state();
    let first_pane = state
        .tab_layout()
        .panes()
        .into_iter()
        .find(|pane| pane.terminal == Some(first.fields().address))
        .unwrap()
        .id;
    state.tab_layout_mut().focus(first_pane).unwrap();

    state.replace_terminals(vec![ended(&first), second.clone()]);

    let panes = state.tab_layout().panes();
    assert_eq!(panes.len(), 1);
    assert_eq!(panes[0].terminal, Some(second.fields().address));
    assert_eq!(state.focused_terminal(), Some(second.fields().address));
    assert_eq!(state.mode, Mode::Terminal);
}

#[test]
fn unreachable_or_output_blocked_terminals_stay_bound() {
    for status in [RuntimeStatus::Unreachable, RuntimeStatus::OutputBlocked] {
        let session = fixtures::session();
        let address = session.fields().address;
        let mut state = state_with(std::slice::from_ref(&session));
        bind_as_terminal_tab(&mut state, address);
        state.mode = Mode::Terminal;
        let mut fields = session.fields().clone();
        fields.runtime.as_mut().unwrap().status = status;
        state.replace_terminals(vec![TerminalSummary::new(fields).unwrap()]);
        assert_eq!(state.focused_terminal(), Some(address), "{status:?}");
    }
}

#[test]
fn exit_event_retires_the_pane_without_waiting_for_the_catalog() {
    let session = fixtures::session();
    let address = session.fields().address;
    let target = session.fields().runtime.unwrap().target;
    let mut state = state_with(std::slice::from_ref(&session));
    bind_as_terminal_tab(&mut state, address);
    state.mode = Mode::Terminal;

    crate::observation_events::terminal(
        &mut state,
        TerminalEvent::Exited {
            cursor: OutputCursor {
                target,
                after_sequence: u64::default(),
            },
            code: Some(i32::default()),
            signal: None,
        },
    );

    assert_eq!(state.focused_terminal(), None);
    assert!(state.ended_epoch(address).is_some());
    assert!(
        state
            .tabs
            .active()
            .view
            .layout
            .panes()
            .iter()
            .all(|pane| pane.terminal.is_none())
    );
}


include!("tab_terminal_retirement.rs");
