use super::{fixtures, panes};
use crate::{Mode, State, pointer};
use crossterm::event::{MouseButton, MouseEventKind};
use yolo_hub_rpc_protocol::HubSummary;
use yolo_tui_inventory::{InventoryHit, InventorySelection, View};

fn state(view: View) -> State {
    let pairs = [fixtures::agent(), fixtures::agent()];
    let mut state = crate::client_capability_support::state();
    state.inventory.replace(
        Some(pairs.iter().map(|(agent, _)| agent.clone()).collect()),
        Some(
            pairs
                .iter()
                .map(|(_, session)| HubSummary {
                    hub_id: session.fields().address.hub_id,
                    reachable: true,
                    label: None,
                })
                .collect(),
        ),
    );
    state.inventory.current = true;
    state.inventory.view = view;
    state.replace_terminals(pairs.into_iter().map(|(_, session)| session).collect());
    if view == View::Terminals {
        crate::tabs::terminal_picker::open(&mut state);
    }
    panes::repaint(&mut state);
    state
}

fn position(state: &State, expected: InventoryHit) -> (u16, u16) {
    let viewport = state.presentation.inventory.as_ref().unwrap();
    let area = viewport.area();
    (area.y..area.bottom())
        .flat_map(|y| (area.x..area.right()).map(move |x| (x, y)))
        .find(|(x, y)| viewport.hit(*x, *y) == Some(expected))
        .unwrap()
}

#[test]
fn one_inventory_click_selects_exact_displayed_resource_and_opens_agent_terminals() {
    for view in [View::Terminals, View::Hubs] {
        let mut state = state(view);
        let layout = state.tab_layout().clone();
        let target = *state
            .inventory
            .visible_selections(&state.filter, &state.terminals)
            .last()
            .unwrap();
        let (x, y) = position(&state, InventoryHit::Row(target));
        let (jobs, mut pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
        pointer::handle(
            &mut state,
            &jobs,
            panes::mouse(MouseEventKind::Down(MouseButton::Left), x, y),
        );
        assert_eq!(state.inventory.selected(state.selected), Some(target));
        assert_eq!(state.mode, Mode::Inventory);
        assert_eq!(*state.tab_layout(), layout);
        assert!(state.writers.is_empty());
        assert!(pending.try_recv().is_err());
    }

    let mut state = state(View::Agents);
    let target = *state
        .inventory
        .visible_selections(&state.filter, &state.terminals)
        .last()
        .unwrap();
    let InventorySelection::Agent(agent_target) = target else {
        panic!()
    };
    let agent_terminal = state.inventory.agents[&agent_target]
        .fields()
        .terminal
        .address;
    let (x, y) = position(&state, InventoryHit::Row(target));
    let (jobs, _pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        panes::mouse(MouseEventKind::Down(MouseButton::Left), x, y),
    );
    assert_eq!(state.inventory.selected(state.selected), Some(target));
    assert_eq!(state.focused_terminal(), Some(agent_terminal));
    assert_eq!(state.mode, Mode::Terminal);
}

#[test]
fn inventory_header_changes_view_without_opening_a_terminal_or_grant() {
    let mut state = state(View::Agents);
    let (jobs, mut pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    for view in [View::Hubs, View::Agents] {
        let (x, y) = position(&state, InventoryHit::Tab(view));
        pointer::handle(
            &mut state,
            &jobs,
            panes::mouse(MouseEventKind::Down(MouseButton::Left), x, y),
        );
        assert_eq!(state.inventory.view, view);
        assert_eq!(state.mode, Mode::Inventory);
        assert!(state.focused_terminal().is_none());
        panes::repaint(&mut state);
    }
    assert!(pending.try_recv().is_err());
}

#[test]
fn inventory_wheel_selects_next_row_but_old_frame_cannot_be_reused_for_another_scroll() {
    let mut state = state(View::Terminals);
    let visible = state
        .inventory
        .visible_selections(&state.filter, &state.terminals);
    let rows = state.presentation.inventory.as_ref().unwrap().rows_area();
    let (jobs, mut pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let down = panes::mouse(MouseEventKind::ScrollDown, rows.x, rows.y);
    pointer::handle(&mut state, &jobs, down);
    assert_eq!(
        state.inventory.selected(state.selected),
        visible.last().copied()
    );
    pointer::handle(
        &mut state,
        &jobs,
        panes::mouse(MouseEventKind::ScrollUp, rows.x, rows.y),
    );
    assert_eq!(
        state.inventory.selected(state.selected),
        visible.last().copied()
    );
    panes::repaint(&mut state);
    pointer::handle(
        &mut state,
        &jobs,
        panes::mouse(MouseEventKind::ScrollUp, rows.x, rows.y),
    );
    assert_eq!(
        state.inventory.selected(state.selected),
        visible.first().copied()
    );
    assert!(pending.try_recv().is_err());
}

#[test]
fn agent_wheel_browses_the_list_without_opening_the_selected_terminal() {
    let mut state = state(View::Agents);
    let visible = state
        .inventory
        .visible_selections(&state.filter, &state.terminals);
    let rows = state.presentation.inventory.as_ref().unwrap().rows_area();
    let (jobs, _pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        panes::mouse(MouseEventKind::ScrollDown, rows.x, rows.y),
    );
    assert_eq!(
        state.inventory.selected(state.selected),
        visible.last().copied()
    );
    assert_eq!(state.mode, Mode::Inventory);
    assert!(state.focused_terminal().is_none());
    assert!(state.inventory.details.is_none());
}

#[test]
fn catalogue_filter_and_execution_changes_cannot_retarget_an_old_row_click() {
    for changed_filter in [false, true] {
        let mut state = state(View::Terminals);
        let target = *state.terminals.keys().last().unwrap();
        let (x, y) = position(
            &state,
            InventoryHit::Row(InventorySelection::Terminal(target)),
        );
        let before = state.selected;
        if changed_filter {
            state.filter = fixtures::SESSION_DIRECTORY.into();
        } else {
            let mut session = state.terminals[&target].fields().clone();
            session.runtime.as_mut().unwrap().target.runtime_epoch =
                yolo_hub_identity::RuntimeEpoch::new();
            state.terminals.insert(
                target,
                yolo_hub_terminal_protocol::TerminalSummary::new(session).unwrap(),
            );
        }
        let (jobs, mut pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
        pointer::handle(
            &mut state,
            &jobs,
            panes::mouse(MouseEventKind::Down(MouseButton::Left), x, y),
        );
        assert_eq!(state.selected, before);
        assert!(pending.try_recv().is_err());
    }
}
