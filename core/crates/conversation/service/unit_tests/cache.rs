use super::*;
use crate::cache::CachedPage;
use crate::history::History;
use yolo_conversation_transcript::{PageSelection, TranscriptSnapshot};
use yolo_crypto::Sha256Digest;

fn page() -> ConversationPage {
    let (binding, target) = binding();
    ConversationPage {
        conversation: resolve_conversation(&binding, target, &sources()).unwrap(),
        history_generation: Sha256Digest::repeated(1),
        revision: Sha256Digest::repeated(2),
        availability: ConversationAvailability::Available,
        partial: Vec::new(),
        entries: Vec::new(),
        next_cursor: None,
    }
}

#[test]
fn unchanged_cache_returns_only_revision_and_other_readers_get_a_page() {
    let page = page();
    let cache = CachedPage {
        stamp: Sha256Digest::repeated(1),
        key: Sha256Digest::repeated(1),
        page: page.clone(),
    };
    assert!(matches!(
        cache.read(Some(page.revision), Some(page.history_generation)),
        ConversationReadResult::Unchanged { .. }
    ));
    assert_eq!(
        cache.read(None, None),
        ConversationReadResult::Page { page: page.clone() }
    );
    assert_eq!(
        cache.read(Some(page.revision), Some(Sha256Digest::repeated(9))),
        ConversationReadResult::Page { page }
    );
}

#[test]
fn cached_projection_memory_is_bounded_separately_from_history_checkpoints() {
    let page = page();
    let address = page.conversation.address.clone();
    let stamp = Sha256Digest::repeated(1);
    let mut history = History::default();
    let snapshot = TranscriptSnapshot {
        length: 0,
        digest: String::new(),
        previous_prefix_digest: String::new(),
        selection: PageSelection::new(0, ConversationLimits::default()),
    };
    history.cache_page(
        &address,
        CachedPage {
            stamp,
            key: stamp,
            page: page.clone(),
        },
    );
    assert!(history.get(&address).is_none());
    history.record(&address, stamp, &snapshot);
    history.cache_page(
        &address,
        CachedPage {
            stamp,
            key: stamp,
            page: page.clone(),
        },
    );
    assert!(history.get(&address).unwrap().cache.is_some());
    for number in 0..17 {
        let mut next = address.clone();
        next.conversation_id = Sha256Digest::capture(&number).unwrap();
        history.record(&next, stamp, &snapshot);
        history.cache_page(
            &next,
            CachedPage {
                stamp,
                key: stamp,
                page: page.clone(),
            },
        );
    }
    assert!(history.get(&address).unwrap().cache.is_none());
}
