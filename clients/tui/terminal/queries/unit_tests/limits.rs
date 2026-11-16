use crate::fixtures::*;
use yolo_tui_terminal_queries::{MAX_REPLY_BYTES, MAX_REPLY_COUNT};

#[test]
fn reply_count_and_byte_limits_bound_adversarial_query_floods() {
    let mut fixture = Fixture::new();
    let mut bytes = DSR.repeat(MAX_REPLY_COUNT + SPLIT_STEP);
    bytes.extend_from_slice(OUTPUT_SUFFIX);
    let replies = fixture.live(&bytes);
    assert_eq!(replies.len(), MAX_REPLY_COUNT);
    assert!(replies.iter().all(|reply| reply == STATUS));
    assert_eq!(fixture.parser.screen().contents(), OUTPUT_SUFFIX_TEXT);
    let mut fixture = Fixture::new();
    let replies = fixture.live(&DA.repeat(MAX_REPLY_BYTES));
    assert_eq!(replies.len(), MAX_REPLY_BYTES / ATTRIBUTES.len());
    assert!(replies.len() <= MAX_REPLY_COUNT);
    assert!(replies.iter().map(Vec::len).sum::<usize>() <= MAX_REPLY_BYTES);
    // A capped chunk never stores a deferred response for the next chunk.
    assert!(fixture.live(&[]).is_empty());
    assert_eq!(fixture.live(DSR), [STATUS.to_vec()]);
}
