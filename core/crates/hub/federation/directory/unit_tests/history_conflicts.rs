use crate::{history_support::*, support::*};
use yolo_hub_federation_directory::OriginBook;

#[test]
fn equal_revision_conflict_rejects_the_entire_merge_after_an_earlier_valid_insertion() {
    let entry = Identity::new();
    let (new_origin, existing_origin) = ordered_origins();
    let current = catalogue(&existing_origin);
    let mut conflicting = current.payload.clone();
    conflicting.policy_epoch += STEP;
    let before = history(entry.identity, vec![current]);
    let incoming = history(
        entry.identity,
        vec![
            catalogue(&new_origin),
            resign(conflicting, &existing_origin),
        ],
    );
    let mut book = offline(entry.identity);
    book.restore_history(before.clone()).unwrap();
    assert!(book.restore_history(incoming).is_err());
    assert_eq!(book.history(), before);
    assert!(book.latest_for(new_origin.identity.hub()).is_none());
}

#[test]
fn conflicting_unpinned_keys_reject_the_entire_merge_in_both_orders() {
    let entry = Identity::new();
    let (new_origin, existing_origin) = ordered_origins();
    let first = catalogue(&existing_origin);
    let replacement = rotate(existing_origin.identity);
    let second = catalogue(&replacement);
    for (retained, conflicting) in [(first.clone(), second.clone()), (second, first)] {
        let before = history(entry.identity, vec![retained]);
        let incoming = history(entry.identity, vec![catalogue(&new_origin), conflicting]);
        let mut book = offline(entry.identity);
        book.restore_history(before.clone()).unwrap();
        assert!(book.restore_history(incoming).is_err());
        assert_eq!(book.history(), before);
        assert!(book.latest_for(new_origin.identity.hub()).is_none());
    }
}

#[test]
fn individually_valid_histories_cannot_overflow_the_merged_capacity_or_replace_existing_state() {
    let entry = Identity::new();
    let (first, second) = ordered_origins();
    let before = history(entry.identity, vec![catalogue(&first)]);
    let incoming = history(entry.identity, vec![catalogue(&second)]);
    let mut book =
        OriginBook::new(entry.identity, Vec::new(), origin_limit(SINGLE_ORIGIN)).unwrap();
    book.restore_history(before.clone()).unwrap();
    assert!(book.restore_history(incoming).is_err());
    assert_eq!(book.history(), before);
    assert!(book.latest_for(second.identity.hub()).is_none());
    assert!(book.exportable(NOW).unwrap().is_empty());
}
