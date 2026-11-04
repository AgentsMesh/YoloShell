use super::{chat_projection::*, chat_support::*};
use crate::{
    Mode,
    chat::{self, Key, network::Reading},
    network::Job,
};
use yolo_conversation_protocol::*;
use yolo_tui_chat::{QueryError, QueryKind, ReadKind};

const OLDER_OFFSET: u64 = 100;

#[test]
fn resolve_latest_older_refresh_and_history_error_keep_query_identity_and_bounded_progress() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    let descriptor = resolve(&mut state, &jobs, &mut incoming);
    let first = operation(&mut state, &jobs, &mut incoming);
    assert!(
        matches!(first.query.kind(), QueryKind::Read { request, kind: ReadKind::Latest } if request.since_revision.is_none())
    );
    let newest = page(&descriptor, 2, OLDER_OFFSET);
    chat::completed(
        &mut state,
        first,
        Ok(Reading::Page(ConversationReadResult::Page {
            page: newest.clone(),
        })),
    );
    let key = Key::current(&state);
    state.chats.entries.get_mut(&key).unwrap().older = true;
    let older = operation(&mut state, &jobs, &mut incoming);
    assert!(matches!(
        older.query.kind(),
        QueryKind::Read {
            kind: ReadKind::Older,
            ..
        }
    ));
    let past = page(&descriptor, 1, OLDER_OFFSET - 1);
    chat::completed(
        &mut state,
        older,
        Ok(Reading::Page(ConversationReadResult::Page { page: past })),
    );
    assert_eq!(chat::current(&state).unwrap().entries().len(), 2);
    let watch = operation(&mut state, &jobs, &mut incoming);
    let QueryKind::Read { request, .. } = watch.query.kind() else {
        panic!()
    };
    assert_eq!(request.since_revision, Some(newest.revision));
    chat::completed(
        &mut state,
        watch,
        Err(QueryError::Conversation(ConversationError::HistoryChanged)),
    );
    assert!(chat::current(&state).unwrap().entries().is_empty());
    state.chats.entries.get_mut(&key).unwrap().older = true;
    let refresh = operation(&mut state, &jobs, &mut incoming);
    assert!(matches!(
        refresh.query.kind(),
        QueryKind::Read {
            kind: ReadKind::Latest,
            ..
        }
    ));
}

#[test]
fn duplicate_old_connection_closed_view_and_unsent_query_cannot_advance_the_new_view() {
    let (mut state, jobs, mut incoming) = ready();
    chat::pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    toggle(&mut state, &jobs);
    let query = operation(&mut state, &jobs, &mut incoming);
    chat::pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    let mut old = query.clone();
    old.generation = uuid::Uuid::new_v4();
    chat::completed(&mut state, old, Err(QueryError::Disconnected));
    assert!(chat::current(&state).unwrap().pending().is_some());
    chat::completed(&mut state, query.clone(), Err(QueryError::Disconnected));
    chat::pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    chat::completed(&mut state, query.clone(), Err(QueryError::InvalidResponse));
    assert_eq!(
        chat::current(&state).unwrap().error(),
        Some(QueryError::Disconnected)
    );
    state.chats.entries.clear();
    chat::completed(&mut state, query, Err(QueryError::Disconnected));
    chat::pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    state.mode = Mode::Terminal;
    toggle(&mut state, &jobs);
    let key = Key::current(&state);
    for _ in 0..crate::constants::QUEUE_CAPACITY {
        assert!(
            jobs.try_send(Job::Refresh {
                request_id: uuid::Uuid::new_v4()
            })
            .is_ok()
        );
    }
    chat::pump(&mut state, &jobs);
    assert!(state.chats.entries[&key].view.pending().is_none());
    assert_eq!(
        state.chats.entries[&key].view.error(),
        Some(QueryError::Disconnected)
    );
    state.disconnected();
    chat::pump(&mut state, &jobs);
    assert!(state.chats.entries[&key].view.pending().is_none());
}
