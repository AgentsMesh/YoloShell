use super::fixture::*;
use yolo_conversation_protocol::*;
use yolo_tui_chat::*;

#[test]
fn well_formed_reply_for_different_descriptor_or_wrong_query_kind_does_not_replace_history() {
    let (mut chat, _) = ready();
    let mut reply = page(&chat, &[1]);
    let query = chat.read(ReadKind::Latest).unwrap();
    reply.conversation.provider = ConversationProvider::claude_code();
    assert_eq!(
        chat.loaded(&query, ConversationReadResult::Page { page: reply }),
        Completion::Rejected
    );
    assert!(chat.entries().is_empty());
    let mut unresolved = Chat::new(chat.binding()).unwrap();
    unresolved.set_mode(Mode::Chat);
    let query = unresolved.resolve().unwrap();
    assert_eq!(
        unresolved.loaded(
            &query,
            ConversationReadResult::Page {
                page: page(&chat, &[1])
            }
        ),
        Completion::Rejected
    );
}

#[test]
fn empty_snapshot_and_rolling_latest_pages_never_exceed_the_retained_window() {
    let (mut chat, _) = ready();
    let first = page(&chat, &[1, 2]);
    load(&mut chat, first);
    for seed in 2..=(MAX_HISTORY_PAGES as u8 + 2) {
        let next = page(&chat, &[seed, seed + 1]);
        load(&mut chat, next);
    }
    assert_eq!(chat.page_count(), MAX_HISTORY_PAGES);
    assert!(chat.entries().len() <= MAX_HISTORY_PAGES + 1);
    let mut empty = page(&chat, &[]);
    empty.next_cursor = None;
    load(&mut chat, empty);
    assert!(chat.page_count() <= MAX_HISTORY_PAGES);
}

#[test]
fn identical_agent_id_with_changed_terminal_runtime_cannot_receive_an_old_draft() {
    let (mut chat, snapshot) = ready();
    chat.composer.insert(PROMPT).unwrap();
    let mut fields = snapshot.fields().clone();
    fields.terminal.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new();
    assert_eq!(
        chat.submit(&yolo_hub_agent_protocol::AgentSnapshot::new(fields).unwrap()),
        Err(DraftError::WrongTarget)
    );
    assert_eq!(chat.composer.text(), PROMPT);
}
