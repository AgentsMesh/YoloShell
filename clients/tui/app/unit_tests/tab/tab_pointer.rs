use super::panes;
use crate::{Mode, State, pointer, tabs};
use crossterm::event::{MouseButton, MouseEventKind};
use uuid::Uuid;

const OTHER_TAB_NAME: &str = "Other tab";

fn state() -> (State, Uuid, Uuid) {
    let (mut state, _, _) = panes::state();
    let first = state.tabs.active().id;
    state.tabs = state.tab_snapshot();
    let second = state.tabs.create(OTHER_TAB_NAME).unwrap();
    state.activate_tab_collection();
    state.mode = Mode::Terminal;
    panes::repaint(&mut state);
    (state, first, second)
}

fn click(state: &mut State, id: Uuid) {
    let area = state
        .presentation
        .tab_strip
        .iter()
        .find(|tab| tab.id == id)
        .unwrap()
        .area;
    let (jobs, mut pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        state,
        &jobs,
        panes::mouse(MouseEventKind::Down(MouseButton::Left), area.x, area.y),
    );
    assert!(pending.try_recv().is_err());
}

#[test]
fn clicking_a_tab_selects_the_exact_painted_id_without_forwarding_pty_input() {
    let (mut state, first, _) = state();
    // Offline control keeps this a local navigation assertion.
    state.control_generation = None;
    click(&mut state, first);
    assert_eq!(state.tabs.active().id, first);
    assert_eq!(state.mode, Mode::Terminal);
    assert!(!state.tab_layout().panes().is_empty());
}

#[test]
fn removed_tab_or_changed_catalog_cannot_retarget_an_old_tab_click() {
    for remove_clicked in [false, true] {
        let (mut state, first, second) = state();
        if remove_clicked {
            state.tabs.remove(first).unwrap();
        } else {
            state.tabs.create("Added later").unwrap();
        }
        let active = state.tabs.active().id;
        let layout = state.tab_layout().clone();
        click(&mut state, first);
        assert_eq!(state.tabs.active().id, active);
        assert_eq!(*state.tab_layout(), layout);
        assert!(state.tabs.entries().iter().any(|entry| entry.id == second));
    }
}

#[test]
fn layout_or_focus_changes_reject_the_old_tab_frame_until_repainted() {
    for change_mode in [false, true] {
        let (mut state, first, second) = state();
        if change_mode {
            state.mode = Mode::Inventory;
        } else {
            state
                .tab_layout_mut()
                .split(yolo_presentation_layout::Axis::Horizontal, None)
                .unwrap();
        }
        click(&mut state, first);
        assert_eq!(state.tabs.active().id, second);
        state.control_generation = None;
        panes::repaint(&mut state);
        click(&mut state, first);
        assert_eq!(state.tabs.active().id, first);
    }
}

#[test]
fn explicit_terminal_picker_has_no_sidebar_terminal_tab_or_pty_preview() {
    let (mut state, _, _) = state();
    tabs::terminal_picker::open(&mut state);
    panes::repaint(&mut state);
    assert!(state.presentation.panes.is_empty());
    let inventory = state.presentation.inventory.as_ref().unwrap();
    assert!(
        (inventory.area().x..inventory.area().right())
            .all(|x| inventory.hit(x, inventory.area().y).is_none())
    );
}
