use crate::{Mode, State, network::Job, pointer_drag::Drag, pointer_input::Capture};
use crossterm::event::{MouseEvent, MouseEventKind};
use tokio::sync::mpsc;

#[derive(Default)]
pub struct Pointer {
    pub drag: Option<Drag>,
    pub capture: Option<Capture>,
}

pub fn handle(state: &mut State, jobs: &mpsc::Sender<Job>, event: MouseEvent) {
    if !matches!(state.mode, Mode::Inventory | Mode::Terminal | Mode::Chat) {
        state.pointer = Pointer::default();
        return;
    }
    if state.presentation.zoomed != state.pane_view.zoomed {
        state.pointer = Pointer::default();
        return;
    }
    if crate::selection_pointer::handle(state, event) {
        return;
    }
    if let Some(capture) = state.pointer.capture {
        match event.kind {
            // A new press recovers from a release lost outside the outer terminal.
            MouseEventKind::Down(button) if button == capture.button => {
                state.pointer.capture = None
            }
            MouseEventKind::Down(_) => return,
            _ => {
                crate::pointer_input::captured(state, jobs, event);
                return;
            }
        }
    }
    if crate::pointer_drag::handle(state, event) {
        return;
    }
    if matches!(event.kind, MouseEventKind::Up(_) | MouseEventKind::Drag(_)) {
        return;
    }
    if !state.presentation.current(&state.tab_layout(), state.mode) {
        return;
    }
    if crate::tabs::click(state, jobs, event) {
        return;
    }
    if crate::inventory_pointer::handle(state, jobs, event) {
        return;
    }
    if state.mode == Mode::Chat
        && state.presentation.panes.iter().any(|view| {
            view.pane == state.tab_layout().focused()
                && contains(view.inner, event.column, event.row)
        })
    {
        crate::chat::handle(state, jobs, &crossterm::event::Event::Mouse(event));
        return;
    }
    let Some(view) = state
        .presentation
        .panes
        .iter()
        .find(|view| contains(view.outer, event.column, event.row))
        .copied()
    else {
        return;
    };
    if matches!(event.kind, MouseEventKind::Down(_)) {
        state.pointer.capture = None;
        let changed = state.tab_layout().focused() != view.pane || state.mode != Mode::Terminal;
        if let Err(error) = state.tab_layout_mut().focus(view.pane) {
            state.status = error.to_string();
            return;
        }
        if view.terminal.is_some() {
            state.mode = Mode::Terminal;
        }
        // Focus requests ordinary input control, never takeover or click-through.
        if changed {
            if view.terminal.is_some() {
                crate::polling::acquire(state, jobs, false);
            }
            return;
        }
    }
    if state.mode == Mode::Terminal
        && state.tab_layout().focused() == view.pane
        && contains(view.inner, event.column, event.row)
    {
        if crate::pointer_scroll::handle(state, jobs, view.terminal, event.kind) {
            return;
        }
        crate::pointer_input::forward(state, jobs, view, event);
    }
}

pub fn contains(rect: ratatui::layout::Rect, x: u16, y: u16) -> bool {
    rect.width != 0
        && rect.height != 0
        && x >= rect.x
        && y >= rect.y
        && u32::from(x) < u32::from(rect.x) + u32::from(rect.width)
        && u32::from(y) < u32::from(rect.y) + u32::from(rect.height)
}
