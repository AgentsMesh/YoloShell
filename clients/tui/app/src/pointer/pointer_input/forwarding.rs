use crate::State;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Predicts whether the pointer router will deliver this event as terminal
/// input. Local selection, tab chrome, scrollback and stale captures stay
/// outside the terminal input FIFO.
pub(crate) fn forwards(state: &State, event: MouseEvent) -> bool {
    if state.mode != crate::Mode::Terminal
        || !state.presentation.current(&state.tab_layout(), state.mode)
        || state.pointer.drag.is_some()
        || state
            .selection
            .active
            .as_ref()
            .is_some_and(|selection| selection.text.is_dragging())
    {
        return false;
    }
    if event.kind == MouseEventKind::Down(MouseButton::Left)
        && event.modifiers.contains(KeyModifiers::SHIFT)
    {
        return false;
    }
    if let Some(capture) = state.pointer.capture {
        return match event.kind {
            MouseEventKind::Down(button) if button == capture.button => {
                normal_forward(state, event)
            }
            MouseEventKind::Down(_) => false,
            MouseEventKind::Drag(button) | MouseEventKind::Up(button)
                if button == capture.button =>
            {
                super::captured_event(state, capture, event)
                    .is_some_and(|(view, event)| can_forward(state, view, event))
            }
            _ => false,
        };
    }
    if matches!(event.kind, MouseEventKind::Up(_) | MouseEventKind::Drag(_)) {
        return false;
    }
    normal_forward(state, event)
}

fn normal_forward(state: &State, event: MouseEvent) -> bool {
    let view = state
        .presentation
        .panes
        .iter()
        .find(|view| view.pane == state.tab_layout().focused())
        .copied();
    let Some(view) =
        view.filter(|view| crate::pointer::contains(view.inner, event.column, event.row))
    else {
        return false;
    };
    can_forward(state, view, event)
}

fn can_forward(state: &State, view: yolo_tui_render::PaneViewport, event: MouseEvent) -> bool {
    super::payload(state, view, event)
        .is_some_and(|(_, result)| result.is_ok_and(|bytes| !bytes.is_empty()))
}
