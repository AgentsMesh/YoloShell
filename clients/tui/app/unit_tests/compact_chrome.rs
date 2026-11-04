use super::{drawing_support, panes};
use crate::{Mode, State, draw};
use ratatui::{Terminal, backend::TestBackend};
use yolo_hub_terminal_protocol::{
    TerminalSize, TerminalSnapshot, TerminalSnapshotFields, TerminalSummary,
};
use yolo_tui_terminal::TerminalSurface;

#[path = "../test_support/chrome_snapshot.rs"]
mod screenshot;

const COLUMNS: u16 = 100;
const ROWS: u16 = 28;
const STATUS_ROWS: u16 = 1;
const TAB_ROWS: u16 = crate::tabs::TAB_BAR_ROWS;
const DIVIDER_COLUMNS: u16 = 1;
const SHELL: &str = "bash";
const OUTPUT: &[u8] = concat!(
    "~/work/yolo $ codex\r\n\r\n",
    "Inspecting the terminal startup flow...\r\n\r\n",
    "  clients/tui/app/src/draw.rs\r\n",
    "  clients/tui/render/src/terminal.rs\r\n\r\n",
    "Ready for the next instruction.\r\n",
)
.as_bytes();
const OTHER_OUTPUT: &[u8] = b"~/work/api $ cargo test\r\n\r\nrunning 42 tests\r\ntest result: ok. 42 passed; 0 failed\r\n\r\n~/work/api $ ";
const FAILURE: &str = "Connection lost; waiting to reconnect";

fn render(state: &State) -> (Terminal<TestBackend>, crate::presentation::Presentation) {
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
    let mut presentation = Default::default();
    terminal
        .draw(|frame| presentation = draw::draw(frame, state))
        .unwrap();
    (terminal, presentation)
}

fn text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

fn sample() -> State {
    let (mut state, address) = drawing_support::state();
    let mut fields = state.terminals[&address].fields().clone();
    fields.name = SHELL.into();
    state
        .terminals
        .insert(address, TerminalSummary::new(fields).unwrap());
    let (_, presentation) = render(&state);
    let size = drawing_support::size(&presentation, address);
    drawing_support::synchronize(&mut state, address, size);
    let snapshot = TerminalSnapshot::new(TerminalSnapshotFields {
        target: state.terminals[&address].fields().runtime.unwrap().target,
        barrier_sequence: 0,
        size,
        data: OUTPUT.into(),
    })
    .unwrap();
    state
        .surfaces
        .insert(address, TerminalSurface::new(&snapshot));
    state
}

#[test]
fn default_terminal_uses_full_width_with_tab_strip_and_one_status_row() {
    let state = sample();
    let (terminal, view) = render(&state);
    assert_eq!(
        view.bounds,
        ratatui::layout::Rect::new(0, TAB_ROWS, COLUMNS, ROWS - STATUS_ROWS - TAB_ROWS,)
    );
    assert_eq!(view.panes[0].inner, view.bounds);
    assert_eq!(
        state.focused_terminal_size(view.bounds),
        Some(TerminalSize::new(COLUMNS, ROWS - STATUS_ROWS - TAB_ROWS).unwrap())
    );
    let content = text(&terminal);
    assert!(content.contains("~/work/yolo $ codex"));
    assert!(!content.contains("Agent runs"));
    assert!(!content.contains("Hubs"));
    assert!(content.contains(&format!("{SHELL} | Control: this UI")));
    for absent in ["Connected", "Terminals:", "Ctrl-b", "Read Only", "┌", "└"] {
        assert!(!content.contains(absent), "{absent}");
    }
    screenshot::save(&terminal, "yolo-terminal.svg");
}

#[test]
fn healthy_inventory_omits_the_connection_header_and_keeps_exceptions_visible() {
    let mut state = crate::client_capability_support::state();
    state.mode = Mode::Inventory;
    state.sidebar_visible = true;
    state.inventory.view = yolo_tui_inventory::View::Agents;
    state.inventory.replace(Some(Vec::new()), Some(Vec::new()));
    state.inventory.current = true;
    state.control_generation = Some(uuid::Uuid::new_v4());
    let (terminal, _) = render(&state);
    let content = text(&terminal);
    assert!(content.contains("Agent runs | Hubs"), "{content}");
    assert!(!content.contains("Agent runs:"), "{content}");
    assert!(!content.contains("Connected"), "{content}");
    assert!(!content.contains("Disconnected"), "{content}");

    state.control_generation = None;
    let (terminal, _) = render(&state);
    assert!(
        text(&terminal).contains(crate::constants::DISCONNECTED_LABEL),
        "{}",
        text(&terminal)
    );

    state.control_generation = Some(uuid::Uuid::new_v4());
    state.inventory.current = false;
    let (terminal, _) = render(&state);
    assert!(
        text(&terminal).contains(crate::constants::CATALOG_REFRESHING_LABEL),
        "{}",
        text(&terminal)
    );
}

#[test]
fn read_only_and_transport_errors_have_one_clear_status_without_duplicate_chrome() {
    let mut state = sample();
    for writer in state.writers.values() {
        state.control.observe(writer.grant.fields().target);
    }
    state.writers.clear();
    state.status = crate::constants::READ_ONLY_LABEL.into();
    let (terminal, _) = render(&state);
    assert_eq!(
        text(&terminal)
            .matches(crate::constants::READ_ONLY_LABEL)
            .count(),
        1
    );
    let status = terminal
        .backend()
        .buffer()
        .content()
        .chunks(usize::from(COLUMNS))
        .last()
        .unwrap()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(!status.contains(yolo_tui_render::WRITING_LABEL));
    screenshot::save(&terminal, "yolo-observer.svg");
    state.status = FAILURE.into();
    let (terminal, _) = render(&state);
    assert_eq!(text(&terminal).matches(FAILURE).count(), 1);
}

#[test]
fn hidden_sidebar_keeps_split_terminal_width_with_a_single_shared_divider() {
    let (mut state, first, focused) = panes::state();
    state.sidebar_visible = false;
    let (_, view) = render(&state);
    state.presentation = view;
    panes::writable(&mut state, &first, OUTPUT);
    panes::writable(&mut state, &focused, OTHER_OUTPUT);
    let (terminal, view) = render(&state);
    assert_eq!(view.panes[0].inner.height, ROWS - STATUS_ROWS - TAB_ROWS);
    assert_eq!(view.panes[1].inner.height, ROWS - STATUS_ROWS - TAB_ROWS);
    assert_eq!(
        view.panes.iter().map(|pane| pane.inner.width).sum::<u16>(),
        COLUMNS - DIVIDER_COLUMNS
    );
    assert!(!text(&terminal).contains("Connected"));
    screenshot::save(&terminal, "yolo-split.svg");
    crate::tabs::terminal_picker::open(&mut state);
    let (terminal, _) = render(&state);
    assert!(text(&terminal).contains(crate::tabs::TERMINAL_PICKER_TITLE));
    screenshot::save(&terminal, "yolo-session-picker.svg");
}
