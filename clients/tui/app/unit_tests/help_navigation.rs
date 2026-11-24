use super::query_support::Fixture;
use crate::{Mode, State, actions, draw, help, help_constants::*};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
const SMALL_COLUMNS: u16 = 24;
const SMALL_ROWS: u16 = 8;
const LARGE_COLUMNS: u16 = 140;
const LARGE_ROWS: u16 = 60;
const LAST_LINE: &str = "Ordinary q and ? remain shell input.";

fn render(state: &mut State, columns: u16, rows: u16) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(columns, rows)).unwrap();
    terminal
        .draw(|frame| state.presentation = draw::draw(frame, state))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .chunks(usize::from(columns))
        .map(|row| row.iter().map(|cell| cell.symbol()).collect())
        .collect()
}

fn key(state: &mut State, code: KeyCode) {
    assert!(help::handle(state, &Event::Key(code.into())));
}

#[test]
fn common_terminal_help_keeps_close_hint_visible_and_reaches_all_content() {
    let mut state = crate::client_capability_support::state();
    help::open(&mut state);
    let first = render(&mut state, COLUMNS, ROWS);
    assert!(first.last().unwrap().contains(HELP_CLOSE_HINT));
    assert!(!first.join("\n").contains(LAST_LINE));
    key(&mut state, KeyCode::PageDown);
    let next = render(&mut state, COLUMNS, ROWS);
    assert_ne!(first, next);
    assert!(next.last().unwrap().contains(HELP_CLOSE_HINT));
    key(&mut state, KeyCode::End);
    let last = render(&mut state, COLUMNS, ROWS);
    assert!(last.join("\n").contains(LAST_LINE));
    key(&mut state, KeyCode::Home);
    assert_eq!(render(&mut state, COLUMNS, ROWS), first);
}

#[test]
fn resizing_a_scrolled_help_clamps_to_its_new_extent_and_preserves_the_footer() {
    let mut state = crate::client_capability_support::state();
    help::open(&mut state);
    render(&mut state, SMALL_COLUMNS, SMALL_ROWS);
    key(&mut state, KeyCode::End);
    let narrow = render(&mut state, SMALL_COLUMNS, SMALL_ROWS);
    assert!(narrow.last().unwrap().contains(HELP_CLOSE_HINT));
    let large = render(&mut state, LARGE_COLUMNS, LARGE_ROWS);
    assert_eq!(state.presentation.help.unwrap().offset, 0);
    assert!(large.join("\n").contains("Start claude, codex"));
    assert!(large.join("\n").contains(LAST_LINE));
    key(&mut state, KeyCode::Down);
    assert_eq!(state.help_offset, 0);
}

#[test]
fn help_quit_closes_only_help_and_detach_chord_exits_without_terminal_input() {
    for code in [KeyCode::Char(QUIT_KEY), KeyCode::Esc, KeyCode::F(1)] {
        let mut fixture = Fixture::new();
        fixture.state.mode = Mode::Terminal;
        help::open(&mut fixture.state);
        assert!(!actions::handle(
            &mut fixture.state,
            &fixture.jobs,
            fixture.target.address.hub_id,
            Event::Key(code.into())
        ));
        assert!(!fixture.state.help);
        assert_eq!(
            fixture.state.focused_terminal(),
            Some(fixture.target.address)
        );
        fixture.no_jobs();
    }
    let mut fixture = Fixture::new();
    help::open(&mut fixture.state);
    assert!(!actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL))
    ));
    assert!(actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        Event::Key(KeyCode::Char(DETACH_KEY).into())
    ));
    fixture.no_jobs();
}
