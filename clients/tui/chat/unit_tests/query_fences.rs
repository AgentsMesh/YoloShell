use super::fixture::*;
use yolo_conversation_protocol::*;
use yolo_tui_chat::*;

#[test]
fn terminal_default_and_mode_epochs_fence_late_resolve_replies() {
    let agent = agent();
    let binding = binding(&agent);
    let mut chat = Chat::new(binding).unwrap();
    assert_eq!(chat.mode(), Mode::Terminal);
    assert_eq!(chat.binding(), binding);
    assert!(chat.resolve().is_none());
    assert!(chat.read(ReadKind::Latest).is_none());
    chat.set_mode(Mode::Chat);
    let old = chat.resolve().unwrap();
    assert!(chat.resolve().is_none());
    chat.set_mode(Mode::Terminal);
    chat.set_mode(Mode::Chat);
    let current = chat.resolve().unwrap();
    assert_ne!(old.id(), current.id());
    assert_eq!(
        chat.resolved(&old, descriptor(binding)),
        Completion::Ignored
    );
    assert_eq!(chat.pending(), Some(&current));
    chat.set_mode(Mode::Chat);
    assert_eq!(
        chat.resolved(&current, descriptor(binding)),
        Completion::Applied
    );
    assert!(chat.resolve().is_none());
    let mut invalid = binding;
    invalid.runtime.address.hub_id = yolo_hub_identity::HubId::new();
    assert!(Chat::new(invalid).is_err());
}

#[test]
fn cross_execution_and_wrong_reply_kinds_never_update_a_view() {
    let (mut chat, _) = ready();
    let request = chat.read(ReadKind::Latest).unwrap();
    let other = descriptor(binding(&agent()));
    assert_eq!(chat.resolved(&request, other), Completion::Rejected);
    assert_eq!(chat.error(), Some(QueryError::InvalidResponse));
    let request = chat.read(ReadKind::Latest).unwrap();
    let mut response = page(&chat, &[1]);
    response.conversation.binding = binding(&agent());
    assert_eq!(
        chat.loaded(&request, ConversationReadResult::Page { page: response }),
        Completion::Rejected
    );
    assert!(chat.entries().is_empty());
    let mut chat = Chat::new(binding(&agent())).unwrap();
    chat.set_mode(Mode::Chat);
    let request = chat.resolve().unwrap();
    assert_eq!(
        chat.resolved(&request, descriptor(binding(&agent()))),
        Completion::Rejected
    );
}

#[test]
fn read_responses_are_scoped_to_the_exact_query_and_generation() {
    let (mut chat, _) = ready();
    let response = page(&chat, &[1]);
    let old = chat.read(ReadKind::Latest).unwrap();
    assert!(chat.read(ReadKind::Latest).is_none());
    chat.set_mode(Mode::Terminal);
    chat.set_mode(Mode::Chat);
    let current = chat.read(ReadKind::Latest).unwrap();
    assert_eq!(
        chat.loaded(
            &old,
            ConversationReadResult::Page {
                page: response.clone()
            }
        ),
        Completion::Ignored
    );
    assert_eq!(
        chat.fail(&old, QueryError::Disconnected),
        Completion::Ignored
    );
    assert_eq!(
        chat.loaded(&current, ConversationReadResult::Page { page: response }),
        Completion::Applied
    );
    let refresh = chat.read(ReadKind::Latest).unwrap();
    let QueryKind::Read { request, .. } = refresh.kind() else {
        panic!()
    };
    assert!(request.cursor.is_none());
    let unchanged = ConversationReadResult::Unchanged {
        history_generation: request.since_generation.unwrap(),
        revision: request.since_revision.unwrap(),
    };
    assert_eq!(chat.loaded(&refresh, unchanged), Completion::Applied);
    assert_eq!(chat.entries().len(), 1);
}

#[test]
fn transient_errors_retain_history_but_history_or_binding_changes_reset_their_scope() {
    let (mut chat, _) = ready();
    let response = page(&chat, &[1]);
    load(&mut chat, response);
    chat.scroll(10, true);
    let query = chat.read(ReadKind::Latest).unwrap();
    assert_eq!(
        chat.fail(&query, QueryError::Disconnected),
        Completion::Applied
    );
    assert_eq!(chat.entries().len(), 1);
    let query = chat.read(ReadKind::Latest).unwrap();
    chat.fail(
        &query,
        QueryError::Conversation(ConversationError::HistoryChanged),
    );
    assert!(chat.entries().is_empty());
    assert_eq!(chat.scroll_offset(), 0);
    assert!(chat.descriptor().is_some());
    let query = chat.read(ReadKind::Latest).unwrap();
    chat.fail(
        &query,
        QueryError::Conversation(ConversationError::BindingChanged),
    );
    assert!(chat.descriptor().is_none());
    assert!(chat.resolve().is_some());
}
