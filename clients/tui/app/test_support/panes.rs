use super::fixtures;
use crate::{Mode, State, draw, network::Job};
use ratatui::{Terminal, backend::TestBackend};
use yolo_hub_terminal_protocol::{
    TerminalSize, TerminalSnapshot, TerminalSnapshotFields, TerminalSummary,
};
use yolo_tui_terminal::TerminalSurface;

pub const WIDTH: u16 = 120;
pub const HEIGHT: u16 = 32;
pub const MOVE_CELLS: u16 = 5;
pub const ENABLE_MOUSE: &[u8] = b"\x1b[?1002h\x1b[?1006h";
pub const SGR_FIRST_CLICK: &[u8] = b"\x1b[<0;1;1M";
pub const SGR_FIRST_RELEASE: &[u8] = b"\x1b[<0;1;1m";
pub const BORDER: u16 = 1;

pub fn repaint(state: &mut State) {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    let mut rendered = crate::presentation::Presentation::default();
    terminal
        .draw(|frame| rendered = draw::draw(frame, state))
        .unwrap();
    state.presentation = rendered;
}
pub fn state() -> (State, TerminalSummary, TerminalSummary) {
    let first = fixtures::session();
    let second = fixtures::session();
    let mut state = crate::client_capability_support::state();
    state.replace_terminals(vec![first.clone(), second.clone()]);
    state.bind_focused(Some(first.fields().address)).unwrap();
    state
        .tab_layout_mut()
        .split(
            yolo_presentation_layout::Axis::Horizontal,
            Some(second.fields().address),
        )
        .unwrap();
    state.mode = Mode::Terminal;
    state.control_generation = Some(uuid::Uuid::new_v4());
    repaint(&mut state);
    (state, first, second)
}
pub fn writable(state: &mut State, session: &TerminalSummary, data: &[u8]) {
    let address = session.fields().address;
    let view = state
        .presentation
        .panes
        .iter()
        .find(|pane| pane.terminal == Some(address))
        .unwrap();
    let size = TerminalSize::new(view.inner.width, view.inner.height).unwrap();
    state.record_grant(fixtures::grant(session), std::time::Instant::now());
    state.set_last_size(address, size);
    let snapshot = TerminalSnapshot::new(TerminalSnapshotFields {
        target: session.fields().runtime.unwrap().target,
        barrier_sequence: 0,
        size,
        data: data.to_vec(),
    })
    .unwrap();
    state
        .surfaces
        .insert(address, TerminalSurface::new(&snapshot));
}
pub fn mouse(
    kind: crossterm::event::MouseEventKind,
    column: u16,
    row: u16,
) -> crossterm::event::MouseEvent {
    crossterm::event::MouseEvent {
        kind,
        column,
        row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    }
}
pub fn bytes(job: Job) -> Vec<u8> {
    let Job::UserInput(control) = job else {
        panic!()
    };
    let yolo_hub_rpc_protocol::Command::Input { request } = control.command else {
        panic!()
    };
    request.fields().data.clone()
}
