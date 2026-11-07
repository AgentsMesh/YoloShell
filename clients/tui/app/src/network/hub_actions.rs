use crate::{Mode, State, network::Job};
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_identity::HubId;
use yolo_tui_hub_manager::{Effect, Manager, OPEN_LABEL, Operation};
use yolo_tui_inventory::View;

const MIN_HEADER_SPACE: u16 = 20;

pub fn button(state: &State, header: Rect) -> Option<Rect> {
    let width = u16::try_from(OPEN_LABEL.len()).ok()?;
    if state.mode != Mode::Inventory
        || !state.client_capability_enabled(ClientCapability::HubManagement)
        || state.inventory.view != View::Hubs
        || header.width < width + MIN_HEADER_SPACE
        || header.height == 0
    {
        return None;
    }
    Some(Rect::new(
        header.right().saturating_sub(width),
        header.y,
        width,
        1,
    ))
}

/// The button shares the section row with `Agent runs | Hubs`, so Hub
/// configuration does not cost a separate header line.
pub fn draw_button(frame: &mut Frame<'_>, state: &State, area: Rect) -> Option<Rect> {
    let button = button(state, area)?;
    frame.render_widget(
        Paragraph::new(OPEN_LABEL).style(Style::default().fg(Color::Black).bg(Color::Cyan)),
        button,
    );
    Some(button)
}
pub fn open_event(state: &State, event: &Event) -> bool {
    if state.mode != Mode::Inventory || state.inventory.view != View::Hubs {
        return false;
    }
    match event {
        Event::Key(key) => {
            key.kind != KeyEventKind::Release
                && key.code == KeyCode::Char('m')
                && key.modifiers.is_empty()
        }
        Event::Mouse(mouse) => {
            mouse.kind == MouseEventKind::Down(MouseButton::Left)
                && state.presentation.current(&state.tab_layout(), state.mode)
                && state
                    .presentation
                    .hub_button
                    .is_some_and(|rect| crate::pointer::contains(rect, mouse.column, mouse.row))
        }
        _ => false,
    }
}
pub fn open(state: &mut State, jobs: &mpsc::Sender<Job>, entry: HubId) {
    if !state.client_capability_enabled(ClientCapability::HubManagement) {
        state.status = crate::client_capabilities::HUB_MANAGEMENT_UNAVAILABLE.into();
        return;
    }
    let (manager, operation) = Manager::open(entry);
    state.hub_manager = Some(manager);
    state.mode = Mode::HubConfiguration;
    state.pointer = crate::pointer::Pointer::default();
    state.prefix = false;
    send(state, jobs, operation);
}
pub fn handle(state: &mut State, jobs: &mpsc::Sender<Job>, event: Event) {
    let effect = state.hub_manager.as_mut().and_then(|manager| match event {
        Event::Mouse(mouse) => state
            .presentation
            .hub_view
            .as_ref()
            .and_then(|view| manager.pointer(view, mouse)),
        event => manager.handle(event),
    });
    match effect {
        Some(Effect::Request(operation)) => send(state, jobs, operation),
        Some(Effect::Close) => {
            state.hub_manager = None;
            state.mode = Mode::Inventory;
        }
        None => {}
    }
}
fn send(state: &mut State, jobs: &mpsc::Sender<Job>, operation: Operation) {
    let token = operation.token;
    if !state.client_capability_enabled(ClientCapability::HubManagement) {
        if let Some(manager) = &mut state.hub_manager {
            manager.complete(
                token,
                Err(yolo_hub_management_client::ManagementError::Unsupported),
            );
        }
        state.status = crate::client_capabilities::HUB_MANAGEMENT_UNAVAILABLE.into();
        return;
    }
    if jobs.try_send(Job::Management(operation)).is_err() {
        if let Some(manager) = &mut state.hub_manager {
            manager.not_sent(token);
        }
    }
}
