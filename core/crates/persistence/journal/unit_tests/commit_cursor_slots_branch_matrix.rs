use super::*;

const INVALID_CURSOR_SLOT: usize = RETAINED_CURSOR_SLOT_COUNT;
const SECOND_CURSOR_SLOT_OFFSET: u64 = COMMIT_CURSOR_BYTES as u64;
const PARTIAL_CURSOR_BYTES: u64 = SECOND_CURSOR_SLOT_OFFSET + CURSOR_REVISION_INCREMENT;
const EMPTY_CURSOR_BYTES: u64 = 0;

#[test]
fn stage_plan_selects_the_older_slot_in_both_revision_orders() {
    let generation = Uuid::new_v4();
    for (first_revision, second_revision, expected_offset) in [
        (EARLIER_REVISION, LATER_REVISION, FIRST_CURSOR_SLOT_OFFSET),
        (LATER_REVISION, EARLIER_REVISION, SECOND_CURSOR_SLOT_OFFSET),
    ] {
        let cursor = cursor_file(
            record(generation, EARLIER_COMMITTED_BYTES, first_revision),
            record(generation, LATER_COMMITTED_BYTES, second_revision),
        );
        let plan = plan_retained_stage(
            &cursor.file,
            cursor.identity,
            Path::new(CURSOR_TEST_PATH),
            record(generation, LATER_COMMITTED_BYTES, LATER_REVISION),
            LATER_COMMITTED_BYTES,
        )
        .unwrap();
        assert_eq!(plan.writes[FIRST_WRITE_INDEX].offset, expected_offset);
    }
}

#[test]
fn slot_validation_covers_mixed_conflicting_and_partial_shapes() {
    let generation = Uuid::new_v4();
    let other_generation = Uuid::new_v4();
    let first = record(generation, EARLIER_COMMITTED_BYTES, LATER_REVISION);
    let matching = record(generation, LATER_COMMITTED_BYTES, LATER_REVISION);
    let mixed = record(other_generation, LATER_COMMITTED_BYTES, EARLIER_REVISION);
    assert!(
        CursorSlots {
            records: [Some(first), Some(matching)]
        }
        .validate(false)
        .is_err()
    );
    assert!(
        CursorSlots {
            records: [Some(first), Some(mixed)]
        }
        .validate(false)
        .is_err()
    );
    assert!(
        CursorSlots {
            records: [Some(first), Some(mixed)]
        }
        .validate(true)
        .is_ok()
    );
    for records in [[Some(first), None], [None, Some(first)]] {
        assert!(CursorSlots { records }.validate(false).is_ok());
    }
    assert!(
        CursorSlots {
            records: [None, None]
        }
        .validate(false)
        .is_err()
    );
}

#[test]
fn rewrite_and_offset_helpers_reject_empty_or_invalid_slots() {
    assert!(
        CursorSlots {
            records: [None, None]
        }
        .rewrite_slot()
        .is_err()
    );
    assert!(other_slot(INVALID_CURSOR_SLOT).is_err());
    assert!(slot_offset(usize::MAX).is_err());
}

#[test]
fn strict_reader_rejects_partial_and_out_of_range_lengths() {
    let generation = Uuid::new_v4();
    let first = record(generation, EARLIER_COMMITTED_BYTES, EARLIER_REVISION);
    let second = record(generation, LATER_COMMITTED_BYTES, LATER_REVISION);
    let partial = cursor_file(first, second);
    partial.file.set_len(PARTIAL_CURSOR_BYTES).unwrap();
    let identity = inspect(&partial.file);
    assert!(read_active_cursor(&partial.file, identity, Path::new(CURSOR_TEST_PATH)).is_err());
    assert!(
        plan_retained_stage(
            &partial.file,
            identity,
            Path::new(CURSOR_TEST_PATH),
            first,
            LATER_COMMITTED_BYTES,
        )
        .is_ok()
    );

    partial.file.set_len(EMPTY_CURSOR_BYTES).unwrap();
    let identity = inspect(&partial.file);
    assert!(read_active_cursor(&partial.file, identity, Path::new(CURSOR_TEST_PATH)).is_err());
}
