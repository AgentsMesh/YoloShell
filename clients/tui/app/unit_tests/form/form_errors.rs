use crate::{Mode, State, actions, constants::QUEUE_CAPACITY, form_input};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, text::Span};
use tokio::sync::mpsc;

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;
const ERROR: &str = "The directory does not exist or is not accessible. Edit the path and retry.";
const DIRECTORY_PREFIX: &str = "/a/long/directory/component";
const DIRECTORY_SUFFIX: &str = "/需要 修改";

fn render(state: &State) -> (String, ratatui::layout::Position) {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal
        .draw(|frame| {
            crate::draw::draw(frame, state);
        })
        .unwrap();
    // Wide glyphs occupy two cells; the second cell is storage padding, not a
    // visible space. Preserve actual spaces while decoding the rendered rows.
    let mut text = String::new();
    for row in terminal
        .backend()
        .buffer()
        .content()
        .chunks(usize::from(WIDTH))
    {
        let mut column = 0;
        while column < row.len() {
            let symbol = row[column].symbol();
            text.push_str(symbol);
            column += Span::raw(symbol).width().max(1);
        }
        text.push('\n');
    }
    (text, terminal.get_cursor_position().unwrap())
}

#[test]
fn failed_directory_keeps_full_error_separate_from_an_editable_scrolled_path() {
    let mut state = crate::client_capability_support::state();
    state.mode = Mode::Directory;
    state.entry = format!("{}{DIRECTORY_SUFFIX}", DIRECTORY_PREFIX.repeat(8));
    form_input::fail(&mut state, ERROR.into());
    let (text, cursor) = render(&state);
    assert!(text.contains(ERROR));
    assert!(text.contains(DIRECTORY_SUFFIX), "{text}");
    assert!(cursor.x < WIDTH);
    assert!(cursor.y < HEIGHT - 1);
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    actions::handle(
        &mut state,
        &jobs,
        yolo_hub_identity::HubId::new(),
        Event::Key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL)),
    );
    assert!(state.entry.is_empty());
    assert!(!render(&state).0.contains(ERROR));
    assert_eq!(
        form_input::rows(&state, WIDTH),
        crate::constants::STATUS_ROWS
    );
    assert!(incoming.try_recv().is_err());
}

#[test]
fn cancelling_or_reopening_a_form_drops_only_its_local_error() {
    for cancel in [false, true] {
        let mut state = crate::client_capability_support::state();
        state.mode = Mode::Directory;
        state.entry = DIRECTORY_SUFFIX.into();
        form_input::fail(&mut state, ERROR.into());
        if cancel {
            let (jobs, _) = mpsc::channel(QUEUE_CAPACITY);
            actions::handle(
                &mut state,
                &jobs,
                yolo_hub_identity::HubId::new(),
                Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            );
            state.mode = Mode::Directory;
        } else {
            form_input::reset(&mut state);
        }
        assert!(!render(&state).0.contains(ERROR));
        assert_eq!(state.entry, DIRECTORY_SUFFIX);
    }
}
