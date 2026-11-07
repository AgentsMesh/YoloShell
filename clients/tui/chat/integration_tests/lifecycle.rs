use super::{fixture::*, render_support};
use yolo_conversation_protocol::*;
use yolo_hub_agent_protocol::CommandOutcome;
use yolo_tui_chat::*;

#[test]
fn terminal_chat_switch_preserves_draft_and_does_not_adopt_late_history_or_retry_input() {
    let (mut chat, agent) = ready();
    let initial = page(&chat, &[1]);
    load(&mut chat, initial);
    chat.composer.insert(PROMPT).unwrap();
    let pending = chat.read(ReadKind::Latest).unwrap();
    let later = page(&chat, &[1, 2]);
    chat.set_mode(Mode::Terminal);
    assert_eq!(
        chat.loaded(&pending, ConversationReadResult::Page { page: later }),
        Completion::Ignored
    );
    assert_eq!(chat.composer.text(), PROMPT);
    chat.set_mode(Mode::Chat);
    let submission = chat.submit(&agent).unwrap();
    chat.set_mode(Mode::Terminal);
    chat.composer.disconnected();
    chat.set_mode(Mode::Chat);
    assert_eq!(chat.submit(&agent), Err(DraftError::Locked));
    assert!(
        render_support::render(&chat, WIDTH, HEIGHT)
            .0
            .contains("Delivery unknown")
    );
    chat.composer
        .received(receipt(&submission, CommandOutcome::Delivered {}));
    assert!(chat.composer.text().is_empty());
    let fresh = page(&chat, &[1, 2, 3]);
    load(&mut chat, fresh);
    assert!(
        render_support::render(&chat, WIDTH, HEIGHT)
            .0
            .contains("message 3")
    );
    assert_eq!(chat.binding().agent, submission.fields().target);
}

#[test]
fn two_clients_keep_their_own_mode_draft_and_scrollback_for_the_same_execution() {
    let (mut first, agent) = ready();
    let mut second = Chat::new(binding(&agent)).unwrap();
    first.composer.insert(PROMPT).unwrap();
    first.scroll(10, true);
    assert_eq!(second.mode(), Mode::Terminal);
    second.set_mode(Mode::Chat);
    let query = second.resolve().unwrap();
    second.resolved(&query, descriptor(binding(&agent)));
    second.composer.insert(FOLLOW_UP).unwrap();
    assert_eq!(first.composer.text(), PROMPT);
    assert_eq!(second.composer.text(), FOLLOW_UP);
    assert_eq!(second.scroll_offset(), 0);
    assert_eq!(first.scroll_offset(), 10);
}
