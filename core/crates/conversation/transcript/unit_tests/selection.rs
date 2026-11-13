use super::*;

fn entry(text: &str) -> ParsedRecord {
    TestParser.parse(&json!({"text":text}))
}

#[test]
fn latest_page_retains_order_and_an_older_cursor_without_unbounded_memory() {
    let mut selection = PageSelection::new(
        50,
        ConversationLimits {
            entries: 2,
            bytes: 1000,
        },
    );
    for offset in 0..6 {
        selection.accept(offset * 10, offset * 10 + 10, entry("test"));
    }
    assert_eq!(
        selection
            .entries
            .iter()
            .map(|(offset, _)| *offset)
            .collect::<Vec<_>>(),
        vec![30, 40]
    );
    assert!(selection.has_older);
}

#[test]
fn clear_discards_previous_history_even_when_reading_an_older_page() {
    let mut selection = PageSelection::new(10, ConversationLimits::default());
    selection.accept(0, 10, entry("private old text"));
    selection.issue(ConversationPartialReason::MalformedRecord);
    selection.accept(10, 20, TestParser.parse(&json!({"clear":true})));
    assert!(selection.entries.is_empty());
    assert!(selection.partial.is_empty());
    assert_eq!(selection.floor, 20);
}

#[test]
fn ignored_provider_records_do_not_mark_history_unavailable() {
    let mut selection = PageSelection::new(u64::MAX, ConversationLimits::default());
    selection.accept(
        0,
        10,
        TestParser.parse(&json!({"ignored": true, "unknown": true, "text": "provider metadata"})),
    );
    assert!(selection.entries.is_empty());
    assert!(selection.partial.is_empty());
    selection.accept(10, 20, TestParser.parse(&json!({"unknown": true})));
    assert_eq!(
        selection.partial,
        vec![ConversationPartialReason::UnsupportedRecord]
    );
}

#[test]
fn byte_limits_and_partial_reasons_remain_explicit() {
    let mut selection = PageSelection::new(
        u64::MAX,
        ConversationLimits {
            entries: 2,
            bytes: 1,
        },
    );
    selection.accept(0, 10, entry("oversized"));
    selection.accept(
        10,
        20,
        TestParser.parse(&json!({"unknown":true,"branch":true})),
    );
    selection.issue(ConversationPartialReason::PageByteLimit);
    assert_eq!(selection.partial.len(), 3);
    assert!(selection.entries.is_empty());
    let mut selection = PageSelection::new(
        u64::MAX,
        ConversationLimits {
            entries: 10,
            bytes: 300,
        },
    );
    selection.accept(0, 10, entry("small"));
    selection.accept(10, 20, entry("small"));
    assert_eq!(selection.entries.len(), 1);
    assert!(selection.has_older);
}
