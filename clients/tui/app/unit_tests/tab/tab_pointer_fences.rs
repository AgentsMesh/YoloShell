use super::panes;
use crate::{Mode, tabs};
use crossterm::event::{KeyModifiers, MouseButton, MouseEventKind};

const OTHER_TAB: &str = "Pointer destination";
const NEXT_ROW: u16 = 1;

#[test]
fn tab_bar_consumes_non_selection_mouse_events_without_retargeting_input() {
    for (kind, modifiers) in [
        (MouseEventKind::Down(MouseButton::Right), KeyModifiers::NONE),
        (
            MouseEventKind::Down(MouseButton::Middle),
            KeyModifiers::NONE,
        ),
        (MouseEventKind::Up(MouseButton::Left), KeyModifiers::NONE),
        (MouseEventKind::Drag(MouseButton::Left), KeyModifiers::NONE),
        (MouseEventKind::ScrollUp, KeyModifiers::NONE),
        (MouseEventKind::Down(MouseButton::Left), KeyModifiers::SHIFT),
        (
            MouseEventKind::Down(MouseButton::Left),
            KeyModifiers::CONTROL,
        ),
    ] {
        let (mut state, _, _) = panes::state();
        let target = state.tabs.active().id;
        state.tabs = state.tab_snapshot();
        let active = state.tabs.create(OTHER_TAB).unwrap();
        state.activate_tab_collection();
        state.mode = Mode::Terminal;
        panes::repaint(&mut state);
        let tab = state
            .presentation
            .tab_strip
            .iter()
            .find(|tab| tab.id == target)
            .unwrap()
            .area;
        let before = state.tab_layout().clone();
        let (jobs, mut pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
        let mut event = panes::mouse(kind, tab.x, tab.y);
        event.modifiers = modifiers;
        assert!(tabs::click(&mut state, &jobs, event));
        assert_eq!(state.tabs.active().id, active);
        assert_eq!(*state.tab_layout(), before);
        assert!(pending.try_recv().is_err());
    }
}

#[test]
fn clicks_below_the_strip_remain_available_to_the_terminal_and_sidebar_handlers() {
    let (mut state, _, _) = panes::state();
    let tab = state.presentation.tab_strip[0].area;
    let active = state.tabs.active().id;
    let (jobs, mut pending) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let event = panes::mouse(
        MouseEventKind::Down(MouseButton::Left),
        tab.x,
        tab.y + NEXT_ROW,
    );
    assert!(!tabs::click(&mut state, &jobs, event));
    assert_eq!(state.tabs.active().id, active);
    assert!(pending.try_recv().is_err());
}
