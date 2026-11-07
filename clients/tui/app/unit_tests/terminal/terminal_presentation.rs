use super::{fixtures, interrupt_support};
use crate::{Mode, State, draw};
use ratatui::{Terminal, backend::TestBackend};
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_terminal_protocol::TerminalSummary;

const TITLE: &str = "Investigating the login failure";
const COLUMNS: u16 = 120;
const ROWS: u16 = 24;

fn screen(state: &State) -> (String, String) {
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
    terminal
        .draw(|frame| {
            draw::draw(frame, state);
        })
        .unwrap();
    let rows = terminal
        .backend()
        .buffer()
        .content()
        .chunks(usize::from(COLUMNS))
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>();
    (rows.join("\n"), rows.last().unwrap().clone())
}

#[test]
fn selected_agent_and_focused_terminal_use_program_title_without_renaming_terminal() {
    let mut state = interrupt_support::ready();
    let address = state.selected.unwrap();
    let mut fields = state.terminals[&address].fields().clone();
    fields.presentation.title = Some(TITLE.into());
    state.replace_terminals(vec![TerminalSummary::new(fields).unwrap()]);
    let (content, status) = screen(&state);
    assert!(content.contains(TITLE));
    assert!(status.starts_with(TITLE));
    assert_eq!(
        state.terminals[&address].fields().name,
        fixtures::SESSION_NAME
    );
    state.mode = Mode::Terminal;
    assert!(screen(&state).1.starts_with(TITLE));
}

#[test]
fn old_agent_does_not_inherit_replacement_runtime_title() {
    let mut state = interrupt_support::ready();
    let address = state.selected.unwrap();
    let mut fields = state.terminals[&address].fields().clone();
    fields.runtime.as_mut().unwrap().target.runtime_epoch = RuntimeEpoch::new();
    fields.presentation.title = Some(TITLE.into());
    state.inventory.history = true;
    state.replace_terminals(vec![TerminalSummary::new(fields).unwrap()]);
    let agent = state.inventory.agents.values().next().unwrap();
    assert_eq!(
        yolo_tui_inventory::agent_title(agent, &state.terminals),
        agent.fields().provider.kind()
    );
    assert!(!screen(&state).1.contains(TITLE));
}

#[test]
fn blank_title_falls_back_to_shell_name() {
    let terminal = fixtures::session();
    let mut fields = terminal.fields().clone();
    fields.presentation.title = Some(String::new());
    assert_eq!(
        TerminalSummary::new(fields).unwrap().display_title(),
        fixtures::SESSION_NAME
    );
}
