use super::fixture::*;
use yolo_conversation_protocol::*;
use yolo_crypto::Sha256Digest;
use yolo_tui_chat::*;
#[path = "../test_support/render.rs"]
mod render_support;

#[test]
fn older_pages_prepend_and_overlap_updates_have_one_chronological_entry() {
    let (mut chat, _) = ready();
    assert!(!chat.older_available());
    assert!(chat.read(ReadKind::Older).is_none());
    let latest = page(&chat, &[3, 4]);
    load(&mut chat, latest);
    let older = chat.read(ReadKind::Older).unwrap();
    let QueryKind::Read { request, .. } = older.kind() else {
        panic!()
    };
    assert!(request.since_revision.is_none() && request.since_generation.is_none());
    let mut past = page(&chat, &[1, 2, 3]);
    past.next_cursor.as_mut().unwrap().before_offset -= 1;
    past.entries[2].parts = vec![ConversationPart::Text {
        text: "stale overlap".into(),
    }];
    assert_eq!(
        chat.loaded(&older, ConversationReadResult::Page { page: past }),
        Completion::Applied
    );
    let entries = chat.entries();
    assert_eq!(
        entries.iter().map(|entry| entry.id).collect::<Vec<_>>(),
        [1, 2, 3, 4].map(Sha256Digest::repeated)
    );
    assert_eq!(entries[2].parts, entry(3).parts);
    let advanced = page(&chat, &[4, 5]);
    load(&mut chat, advanced);
    assert_eq!(
        chat.entries()
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4, 5].map(Sha256Digest::repeated)
    );
}

#[test]
fn history_rewrite_and_missing_append_overlap_never_stitch_unrelated_windows() {
    let (mut chat, _) = ready();
    let latest = page(&chat, &[1, 2]);
    load(&mut chat, latest);
    chat.scroll(10, true);
    let mut rewritten = page(&chat, &[3]);
    rewritten.history_generation = Sha256Digest::repeated(NEXT_HISTORY_SEED);
    rewritten.next_cursor.as_mut().unwrap().history_generation = rewritten.history_generation;
    load(&mut chat, rewritten);
    assert_eq!(chat.entries().len(), 1);
    assert_eq!(chat.scroll_offset(), 0);
    let distant = page(&chat, &[9]);
    load(&mut chat, distant);
    assert_eq!(chat.entries()[0].id, Sha256Digest::repeated(9));
    assert_eq!(chat.page_count(), 1);
}

#[test]
fn page_window_is_bounded_and_scroll_is_client_local() {
    let (mut chat, _) = ready();
    let latest = page(&chat, &[MAX_HISTORY_PAGES as u8]);
    load(&mut chat, latest);
    for step in 1..MAX_HISTORY_PAGES {
        let query = chat.read(ReadKind::Older).unwrap();
        let mut older = page(&chat, &[(MAX_HISTORY_PAGES - step) as u8]);
        older.next_cursor.as_mut().unwrap().before_offset -= step as u64;
        assert_eq!(
            chat.loaded(&query, ConversationReadResult::Page { page: older }),
            Completion::Applied
        );
    }
    assert_eq!(chat.page_count(), MAX_HISTORY_PAGES);
    assert!(!chat.older_available());
    assert!(chat.read(ReadKind::Older).is_none());
    assert!(
        render_support::render(&chat, WIDTH, HEIGHT)
            .0
            .contains("History window limit reached")
    );
    chat.scroll(usize::MAX, true);
    assert_eq!(chat.scroll_offset(), MAX_RENDER_ROWS);
    chat.scroll(MAX_RENDER_ROWS, false);
    assert_eq!(chat.scroll_offset(), 0);
    chat.scroll(2, true);
    chat.follow_tail();
    assert_eq!(chat.scroll_offset(), 0);
}
