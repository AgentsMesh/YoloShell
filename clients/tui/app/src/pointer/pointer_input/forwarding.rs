use super::Capture;
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
                capture_forward(state, capture, event)
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

fn capture_forward(state: &State, capture: Capture, event: MouseEvent) -> bool {
    let Some(view) = state
        .presentation
        .panes
        .iter()
        .find(|view| view.pane == capture.pane)
        .copied()
    else {
        return false;
    };
    if state.tab_layout().focused() != capture.pane {
        return false;
    }
    let Some(address) = view.terminal else {
        return false;
    };
    let Some(grant) = state.grant(address, std::time::Instant::now()) else {
        return false;
    };
    if !same_authority(grant, capture.grant) {
        return false;
    }
    let mut clamped = event;
    clamped.column = event.column.clamp(
        view.inner.x,
        view.inner
            .right()
            .saturating_sub(crate::constants::PANE_BORDER_CELLS),
    );
    clamped.row = event.row.clamp(
        view.inner.y,
        view.inner
            .bottom()
            .saturating_sub(crate::constants::PANE_BORDER_CELLS),
    );
    can_forward(state, view, clamped)
}

fn can_forward(state: &State, view: yolo_tui_render::PaneViewport, event: MouseEvent) -> bool {
    let Some(address) = view.terminal else {
        return false;
    };
    let Some(grant) = state.grant(address, std::time::Instant::now()) else {
        return false;
    };
    if !state.input_ready(address) {
        return false;
    }
    let Some(surface) = state.surfaces.get(&address) else {
        return false;
    };
    if surface.cursor().target != grant.fields().target
        || surface.screen().size() != (view.inner.height, view.inner.width)
        || surface.screen().scrollback() != 0
        || surface.screen().mouse_protocol_mode() == vt100::MouseProtocolMode::None
    {
        return false;
    }
    let mut relative = event;
    relative.column = event.column.saturating_sub(view.inner.x);
    relative.row = event.row.saturating_sub(view.inner.y);
    yolo_tui_input::encode_mouse(relative, surface.screen()).is_ok()
}

fn same_authority(
    left: yolo_hub_terminal_protocol::LeaseGrant,
    right: yolo_hub_terminal_protocol::LeaseGrant,
) -> bool {
    let (left, right) = (left.fields(), right.fields());
    left.target == right.target
        && left.generation == right.generation
        && left.capability == right.capability
        && left.principal == right.principal
}
