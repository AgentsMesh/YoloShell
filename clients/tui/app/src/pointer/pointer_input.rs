use crate::{State, network::Job, terminal_actions::deliver};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_terminal_protocol::LeaseGrant;
use yolo_tui_render::PaneViewport;

#[path = "pointer_input/forwarding.rs"]
mod forwarding;

pub(crate) use forwarding::forwards;

#[derive(Clone, Copy)]
pub struct Capture {
    pane: Uuid,
    grant: LeaseGrant,
    pub(crate) button: MouseButton,
}

pub fn captured(state: &mut State, jobs: &mpsc::Sender<Job>, event: MouseEvent) {
    let Some(capture) = state.pointer.capture.take() else {
        return;
    };
    if !matches!(event.kind, MouseEventKind::Drag(button) | MouseEventKind::Up(button) if button == capture.button)
    {
        state.pointer.capture = Some(capture);
        return;
    }
    if !state.presentation.current(&state.tab_layout(), state.mode)
        || state.tab_layout().focused() != capture.pane
    {
        return;
    }
    let Some(view) = state
        .presentation
        .panes
        .iter()
        .find(|view| view.pane == capture.pane)
        .copied()
    else {
        return;
    };
    let Some(grant) = view
        .terminal
        .and_then(|address| state.grant(address, std::time::Instant::now()))
    else {
        return;
    };
    if !same_authority(grant, capture.grant) || view.inner.width == 0 || view.inner.height == 0 {
        return;
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
    if forward(state, jobs, view, clamped) && matches!(event.kind, MouseEventKind::Drag(_)) {
        state.pointer.capture = Some(capture);
    }
}

pub fn forward(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    view: PaneViewport,
    event: MouseEvent,
) -> bool {
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
    {
        return false;
    }
    let mut relative = event;
    relative.column = event.column.saturating_sub(view.inner.x);
    relative.row = event.row.saturating_sub(view.inner.y);
    let encoded = yolo_tui_input::encode_mouse(relative, surface.screen());
    let sent = deliver(state, jobs, encoded);
    if sent {
        if let MouseEventKind::Down(button) = event.kind {
            state.pointer.capture = Some(Capture {
                pane: view.pane,
                grant,
                button,
            });
        }
    }
    sent
}

fn same_authority(left: LeaseGrant, right: LeaseGrant) -> bool {
    let (left, right) = (left.fields(), right.fields());
    left.target == right.target
        && left.generation == right.generation
        && left.capability == right.capability
        && left.principal == right.principal
}
