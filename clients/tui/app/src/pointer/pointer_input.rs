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
    let Some((view, clamped)) = captured_event(state, capture, event) else {
        return;
    };
    if forward(state, jobs, view, clamped) && matches!(event.kind, MouseEventKind::Drag(_)) {
        state.pointer.capture = Some(capture);
    }
}

fn captured_event(
    state: &State,
    capture: Capture,
    event: MouseEvent,
) -> Option<(PaneViewport, MouseEvent)> {
    if !state.presentation.current(&state.tab_layout(), state.mode)
        || state.tab_layout().focused() != capture.pane
    {
        return None;
    }
    let view = state
        .presentation
        .panes
        .iter()
        .find(|view| view.pane == capture.pane)
        .copied()?;
    let grant = view
        .terminal
        .and_then(|address| state.grant(address, std::time::Instant::now()))?;
    if !crate::query_state::same_writer(grant, capture.grant)
        || view.inner.width == 0
        || view.inner.height == 0
    {
        return None;
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
    Some((view, clamped))
}

pub fn forward(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    view: PaneViewport,
    event: MouseEvent,
) -> bool {
    let Some((grant, encoded)) = payload(state, view, event) else {
        return false;
    };
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

fn payload(
    state: &State,
    view: PaneViewport,
    mut event: MouseEvent,
) -> Option<(LeaseGrant, Result<Vec<u8>, yolo_tui_input::InputError>)> {
    let address = view.terminal?;
    let grant = state.grant(address, std::time::Instant::now())?;
    let surface = state.surfaces.get(&address)?;
    if !state.input_ready(address)
        || surface.cursor().target != grant.fields().target
        || surface.screen().size() != (view.inner.height, view.inner.width)
        || surface.screen().scrollback() != 0
        || surface.screen().mouse_protocol_mode() == vt100::MouseProtocolMode::None
    {
        return None;
    }
    event.column = event.column.saturating_sub(view.inner.x);
    event.row = event.row.saturating_sub(view.inner.y);
    Some((grant, yolo_tui_input::encode_mouse(event, surface.screen())))
}
