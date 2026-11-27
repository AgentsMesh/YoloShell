use super::fixture::*;
use yolo_tui_chat::{Completion, DraftError, Mode, ReadKind};

#[test]
fn retired_query_epoch_preserves_history_and_draft_but_never_accepts_old_results_or_new_submissions()
 {
    let (mut chat, agent) = ready();
    let first = page(&chat, &[1]);
    load(&mut chat, first);
    chat.composer.insert(PROMPT).unwrap();
    let query = chat.read(ReadKind::Latest).unwrap();
    let late = page(&chat, &[1, 2]);
    chat.retire();
    chat.retire();
    assert!(chat.retired());
    assert_eq!(chat.mode(), Mode::Chat);
    assert_eq!(chat.entries().len(), 1);
    assert_eq!(chat.composer.text(), PROMPT);
    assert_eq!(
        chat.loaded(
            &query,
            yolo_conversation_protocol::ConversationReadResult::Page { page: late }
        ),
        Completion::Ignored
    );
    assert!(chat.read(ReadKind::Latest).is_none());
    assert_eq!(chat.submit(&agent), Err(DraftError::WrongTarget));
}
