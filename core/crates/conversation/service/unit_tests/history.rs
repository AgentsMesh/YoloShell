use super::*;
use crate::history::History;
use yolo_conversation_transcript::{PageSelection, TranscriptSnapshot};
use yolo_crypto::Sha256Digest;

fn scan(length: u64, digest: &str, prefix: &str, floor: u64) -> TranscriptSnapshot {
    let mut selection = PageSelection::new(u64::MAX, ConversationLimits::default());
    selection.floor = floor;
    TranscriptSnapshot {
        length,
        digest: digest.into(),
        previous_prefix_digest: prefix.into(),
        selection,
    }
}

#[test]
fn only_append_with_an_unchanged_prefix_preserves_generation() {
    let (binding, target) = binding();
    let address = resolve_conversation(&binding, target, &sources()).unwrap().address;
    let object = Sha256Digest::repeated(1);
    let mut history = History::default();
    let first = history.record(&address, object, &scan(10, "first", "", 0));
    assert_eq!(
        first,
        history.record(&address, object, &scan(20, "second", "first", 0))
    );
    let rewritten = history.record(&address, object, &scan(30, "third", "changed prefix", 0));
    assert_ne!(first, rewritten);
    let cleared = history.record(&address, object, &scan(40, "fourth", "third", 20));
    assert_ne!(cleared, rewritten);
    let replaced = history.record(
        &address,
        Sha256Digest::repeated(2),
        &scan(40, "fourth", "fourth", 20),
    );
    assert_ne!(replaced, cleared);
    let truncated = history.record(
        &address,
        Sha256Digest::repeated(2),
        &scan(10, "short", "fourth", 0),
    );
    assert_ne!(truncated, replaced);
}

#[test]
fn checkpoint_memory_is_bounded_and_eviction_invalidates_prior_cursor_generation() {
    let (binding, target) = binding();
    let original = resolve_conversation(&binding, target, &sources()).unwrap().address;
    let object = Sha256Digest::repeated(1);
    let mut history = History::default();
    let first = history.record(&original, object, &scan(10, "first", "", 0));
    for number in 0..65 {
        let mut address = original.clone();
        address.conversation_id = Sha256Digest::capture(&json!(number)).unwrap();
        history.record(&address, object, &scan(10, "first", "", 0));
    }
    assert!(history.get(&original).is_none());
    assert_ne!(
        first,
        history.record(&original, object, &scan(10, "first", "first", 0))
    );
}
