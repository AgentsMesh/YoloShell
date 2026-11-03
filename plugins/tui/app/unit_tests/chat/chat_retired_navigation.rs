use super::{chat_projection::*, chat_support::*, drawing_support};
use crate::{
    chat::{self, Key, network::Reading},
    network::Job,
};
use crossterm::event::{KeyCode, KeyModifiers};
use yolo_conversation_protocol::{ConversationPart, ConversationReadResult};

const START: &str = "retained draft first line";
const END: &str = "retained draft last line";
const HISTORY_LINES: usize = 64;
const OFFSET: u64 = 100;

#[test]
fn retired_draft_and_cached_history_remain_navigable_without_querying_or_editing() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    let descriptor = resolve(&mut state, &jobs, &mut incoming);
    let query = operation(&mut state, &jobs, &mut incoming);
    let mut history = page(&descriptor, 1, OFFSET);
    history.entries[0].parts = vec![ConversationPart::Text {
        text: "recorded line\n".repeat(HISTORY_LINES),
    }];
    chat::completed(
        &mut state,
        query,
        Ok(Reading::Page(ConversationReadResult::Page {
            page: history,
        })),
    );
    let draft = format!("{START}\n{}{END}", "middle\n".repeat(HISTORY_LINES));
    paste(&mut state, &jobs, &draft);
    let id = Key::current(&state);
    state.chats.entries.get_mut(&id).unwrap().view.retire();
    assert!(
        drawing_support::render(&state, WIDTH, HEIGHT)
            .0
            .contains(END)
    );
    key(&mut state, &jobs, KeyCode::Home, KeyModifiers::CONTROL);
    assert!(
        drawing_support::render(&state, WIDTH, HEIGHT)
            .0
            .contains(START)
    );
    for code in [KeyCode::Right, KeyCode::Left, KeyCode::End, KeyCode::Home] {
        key(&mut state, &jobs, code, KeyModifiers::NONE);
    }
    key(&mut state, &jobs, KeyCode::PageUp, KeyModifiers::NONE);
    assert!(chat::current(&state).unwrap().scroll_offset() > 0);
    assert!(!state.chats.entries[&id].older);
    key(&mut state, &jobs, KeyCode::PageDown, KeyModifiers::NONE);
    assert_eq!(chat::current(&state).unwrap().scroll_offset(), 0);
    key(&mut state, &jobs, KeyCode::End, KeyModifiers::CONTROL);
    assert!(
        drawing_support::render(&state, WIDTH, HEIGHT)
            .0
            .contains(END)
    );
    for code in [
        KeyCode::Backspace,
        KeyCode::Delete,
        KeyCode::Char('x'),
        KeyCode::Enter,
    ] {
        key(&mut state, &jobs, code, KeyModifiers::NONE);
    }
    paste(&mut state, &jobs, NEXT_PROMPT);
    chat::pump(&mut state, &jobs);
    assert_eq!(chat::current(&state).unwrap().composer.text(), draft);
    assert!(incoming.try_recv().is_err());
}
