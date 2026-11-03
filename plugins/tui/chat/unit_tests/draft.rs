use super::fixture::*;
use yolo_agent_input::MAX_PROMPT_BYTES;
use yolo_tui_chat::*;

#[test]
fn unicode_graphemes_newlines_and_tabs_preserve_byte_boundaries_during_editing() {
    let mut composer = Composer::default();
    assert!(!composer.insert("").unwrap());
    composer.insert(&format!("a{FAMILY}{CJK}\t\nx")).unwrap();
    composer.home();
    assert!(composer.delete().unwrap());
    assert!(composer.backspace().unwrap());
    assert!(composer.backspace().unwrap());
    composer.left();
    assert!(composer.delete().unwrap());
    assert!(composer.backspace().unwrap());
    assert!(composer.backspace().unwrap());
    assert_eq!(composer.text(), "a");
    composer.left();
    assert!(!composer.backspace().unwrap());
    composer.right();
    composer.right();
    composer.insert("\ne\u{301}").unwrap();
    assert!(composer.backspace().unwrap());
    assert_eq!(composer.text(), "a\n");
    composer.home();
    composer.end();
    assert!(!composer.delete().unwrap());
    composer.clear().unwrap();
    assert_eq!(composer.cursor(), 0);
}

#[test]
fn unsafe_paste_and_byte_overflow_leave_the_existing_draft_unchanged() {
    let mut composer = Composer::default();
    composer.insert(PROMPT).unwrap();
    for text in ["\0", "\u{1b}[201~\r", "\r", "\u{7}"] {
        assert_eq!(composer.insert(text), Err(DraftError::UnsafeText));
        assert_eq!(composer.text(), PROMPT);
    }
    assert_eq!(
        composer.insert(&"x".repeat(MAX_PROMPT_BYTES)),
        Err(DraftError::TooLarge)
    );
    composer.clear().unwrap();
    assert!(composer.insert(&"x".repeat(MAX_PROMPT_BYTES)).unwrap());
    assert_eq!(composer.insert("x"), Err(DraftError::TooLarge));
    assert_eq!(composer.text().len(), MAX_PROMPT_BYTES);
    for error in [
        DraftError::Locked,
        DraftError::TooLarge,
        DraftError::UnsafeText,
        DraftError::Empty,
        DraftError::WrongTarget,
        DraftError::Unavailable,
        DraftError::ObservationPending,
    ] {
        assert!(!error.to_string().is_empty());
        assert!(std::error::Error::source(&error).is_none());
    }
}

#[test]
fn in_flight_and_uncertain_submissions_lock_text_but_not_cursor_navigation() {
    let (mut chat, agent) = ready();
    chat.composer.insert(PROMPT).unwrap();
    chat.submit(&agent).unwrap();
    assert_eq!(chat.composer.insert(FOLLOW_UP), Err(DraftError::Locked));
    assert_eq!(chat.composer.backspace(), Err(DraftError::Locked));
    assert_eq!(chat.composer.delete(), Err(DraftError::Locked));
    assert_eq!(chat.composer.clear(), Err(DraftError::Locked));
    chat.composer.home();
    chat.composer.end();
    assert_eq!(chat.composer.cursor(), PROMPT.len());
    assert!(!chat.composer.discard_uncertain());
    chat.composer.disconnected();
    assert!(chat.composer.discard_uncertain());
    assert!(chat.composer.text().is_empty());
    assert!(!chat.composer.locked());
    assert!(!chat.composer.discard_uncertain());
}
