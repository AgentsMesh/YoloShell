use super::{creation_support::Fixture, fixtures};
use crate::{Mode, network::Job, tabs, terminal_actions};
use crossterm::event::KeyCode;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::TerminalAction;
use yolo_presentation_layout::{Axis, Tree};
use yolo_tui_inventory::View;

const SINGLE: usize = 1;
const PAIR: usize = 2;
const CLOSE_PANE: char = 'x';
const NEXT_TAB: &str = "Next tab";

fn prefix(fixture: &mut Fixture, code: char) {
    assert!(!terminal_actions::prefix(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        KeyCode::Char(code),
    ));
}

#[test]
fn split_shortcuts_start_one_shell_in_the_requested_direction_without_duplicating_bindings() {
    for (key, expected) in [
        (tabs::SPLIT_RIGHT_KEY, Axis::Horizontal),
        ('%', Axis::Horizontal),
        (tabs::SPLIT_DOWN_KEY, Axis::Vertical),
        ('"', Axis::Vertical),
    ] {
        for empty in [false, true] {
            let mut fixture = Fixture::new();
            fixture.state.mode = Mode::Terminal;
            if empty {
                fixture
                    .state
                    .tab_layout_mut()
                    .bind(fixture.pane, None)
                    .unwrap();
            }
            prefix(&mut fixture, key);
            assert_eq!(
                fixture.state.tab_layout().panes().len(),
                if empty { SINGLE } else { PAIR }
            );
            if !empty {
                assert!(
                    matches!(fixture.state.tab_layout().document().root, Tree::Split {
                    axis, ..
                } if axis == expected)
                );
                assert_eq!(fixture.original_binding(), Some(fixture.expected));
            }
            let Job::Call {
                origin,
                command: Command::Create { request },
                ..
            } = fixture.incoming.try_recv().unwrap()
            else {
                panic!()
            };
            assert_eq!(origin, fixture.origin);
            if !empty {
                assert_eq!(request.fields().cwd, fixtures::SESSION_DIRECTORY);
            }
            assert!(fixture.incoming.try_recv().is_err());
        }
    }
}

#[test]
fn closing_a_sibling_removes_only_its_view_and_preserves_both_terminals() {
    let mut fixture = Fixture::new();
    let sibling = fixtures::session();
    let address = sibling.fields().address;
    fixture.state.terminals.insert(address, sibling);
    fixture
        .state
        .tab_layout_mut()
        .split(Axis::Horizontal, Some(address))
        .unwrap();
    fixture.state.pane_view.zoomed = true;
    let terminals = fixture.state.terminals.clone();
    prefix(&mut fixture, CLOSE_PANE);
    assert_eq!(fixture.state.tab_layout().panes().len(), SINGLE);
    assert_eq!(fixture.state.focused_terminal(), Some(fixture.expected));
    assert_eq!(fixture.state.terminals, terminals);
    assert!(!fixture.state.pane_view.zoomed);
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn closing_last_pane_or_whole_tab_keeps_processes_and_reveals_another_tab() {
    for key in [CLOSE_PANE, tabs::CLOSE_TAB_KEY] {
        let mut fixture = Fixture::new();
        fixture.state.tabs = fixture.state.tab_snapshot();
        fixture.state.tabs.create(NEXT_TAB).unwrap();
        fixture.state.activate_tab_collection();
        fixture.state.bind_focused(Some(fixture.expected)).unwrap();
        prefix(&mut fixture, key);
        assert_eq!(fixture.state.tabs.entries().len(), SINGLE);
        assert_eq!(fixture.state.tabs.active().id, fixture.tab);
        assert_eq!(fixture.state.focused_terminal(), Some(fixture.expected));
        assert!(fixture.state.terminals.contains_key(&fixture.expected));
        assert!(fixture.incoming.try_recv().is_err());
        prefix(&mut fixture, key);
        assert!(fixture.state.focused_terminal().is_none());
        assert_eq!(fixture.state.mode, Mode::Inventory);
        assert!(fixture.state.sidebar_visible);
        assert_eq!(fixture.state.inventory.view, View::Tree);
        assert!(fixture.state.terminals.contains_key(&fixture.expected));
        assert!(fixture.incoming.try_recv().is_err());
    }
}

#[test]
fn closing_a_pending_shell_destination_prevents_its_late_reply_from_reopening_it() {
    let mut fixture = Fixture::new();
    prefix(&mut fixture, tabs::SPLIT_RIGHT_KEY);
    let Job::Call {
        command: Command::Create { request },
        ..
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!()
    };
    prefix(&mut fixture, CLOSE_PANE);
    assert!(!fixture.state.creations.pending[&request.fields().request_id].binding_active);
    let created = fixture.created();
    crate::creation_events::created(&mut fixture.state, &fixture.jobs, request, created);
    assert_eq!(fixture.state.focused_terminal(), Some(fixture.expected));
    assert!(!fixture.state.bound_terminals().contains(&created.address));
    assert!(fixture.state.initial_control.is_none());
    while let Ok(job) = fixture.incoming.try_recv() {
        assert!(matches!(job, Job::Finalize { .. } | Job::Refresh { .. }));
    }
}

#[test]
fn sidebar_navigation_leaves_picker_and_never_presents_terminal_inventory_as_a_sidebar_tab() {
    let mut fixture = Fixture::new();
    fixture.state.inventory.view = View::Terminals;
    tabs::terminal_picker::open(&mut fixture.state);
    tabs::sidebar(&mut fixture.state);
    assert!(fixture.state.terminal_picker.is_none());
    assert_eq!(fixture.state.inventory.view, View::Tree);
    assert_eq!(fixture.state.mode, Mode::Inventory);
    prefix(&mut fixture, tabs::TOGGLE_SIDEBAR_KEY);
    assert!(!fixture.state.sidebar_visible);
    prefix(&mut fixture, tabs::TOGGLE_SIDEBAR_KEY);
    assert!(fixture.state.sidebar_visible);
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn focused_recovery_shortcut_selects_exact_terminal_and_waits_for_explicit_resume() {
    let mut fixture = Fixture::new();
    let mut fields = fixture.state.terminals[&fixture.expected].fields().clone();
    fields.recovery = Some(yolo_hub_terminal_protocol::TerminalRecoveryHint::Ready);
    fixture.state.terminals.insert(
        fixture.expected,
        yolo_hub_terminal_protocol::TerminalSummary::new(fields).unwrap(),
    );
    let other = fixtures::session();
    fixture.state.selected = Some(other.fields().address);
    fixture
        .state
        .terminals
        .insert(other.fields().address, other);
    prefix(&mut fixture, tabs::RESUME_KEY);
    assert!(fixture.state.terminal_picker.is_some());
    assert_eq!(fixture.state.selected, Some(fixture.expected));
    assert!(fixture.state.inventory.history);
    assert!(fixture.incoming.try_recv().is_err());
    crate::inventory_actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        KeyCode::Char(tabs::RESUME_KEY),
    );
    assert!(matches!(fixture.incoming.try_recv().unwrap(), Job::Call {
        command: Command::Terminal { command }, ..
    } if command.selection.address == fixture.expected && command.action == TerminalAction::Resume {}));
}
