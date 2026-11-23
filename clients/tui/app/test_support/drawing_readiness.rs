use super::fixtures;
use crate::{Mode, State, draw, presentation::Presentation};
use ratatui::{Terminal, backend::TestBackend};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::{
    TerminalSize, TerminalSnapshot, TerminalSnapshotFields, TerminalSummary,
};
use yolo_tui_terminal::TerminalSurface;

#[path = "drawing_constants.rs"]
mod constants;
pub use constants::*;

pub fn state() -> (State, TerminalAddress) {
    let mut state = crate::client_capability_support::state();
    state.mode = Mode::Terminal;
    state.control_generation = Some(uuid::Uuid::new_v4());
    let source = session(SOURCE_NAME);
    let address = source.fields().address;
    state.replace_terminals(vec![source.clone()]);
    state.bind_focused(Some(address)).unwrap();
    state.record_grant(fixtures::grant(&source), std::time::Instant::now());
    (state, address)
}

pub fn session(name: &str) -> TerminalSummary {
    let mut fields = fixtures::session().fields().clone();
    fields.name = name.into();
    TerminalSummary::new(fields).unwrap()
}

pub fn render(state: &State, width: u16, height: u16) -> (String, Presentation) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let mut presentation = Presentation::default();
    terminal
        .draw(|frame| presentation = draw::draw(frame, state))
        .unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    (text, presentation)
}

pub fn size(presentation: &Presentation, address: TerminalAddress) -> TerminalSize {
    let inner = presentation
        .panes
        .iter()
        .find(|pane| pane.terminal == Some(address))
        .unwrap()
        .inner;
    TerminalSize::new(inner.width, inner.height).unwrap()
}

pub fn synchronize(state: &mut State, address: TerminalAddress, size: TerminalSize) {
    let target = state.terminals[&address].fields().runtime.unwrap().target;
    let snapshot = TerminalSnapshot::new(TerminalSnapshotFields {
        target,
        barrier_sequence: ZERO_SEQUENCE,
        size,
        data: Vec::new(),
    })
    .unwrap();
    state
        .surfaces
        .insert(address, TerminalSurface::new(&snapshot));
    state.set_last_size(address, size);
    state.clear_resize_pending(address);
    state.clear_resync(address);
}

pub fn label(name: &str, status: &str) -> String {
    format!("{name}{LABEL_SEPARATOR}{status}")
}
