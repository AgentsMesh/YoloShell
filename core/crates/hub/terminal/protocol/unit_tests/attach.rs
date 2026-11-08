use crate::fixtures::{self, constants::*, roundtrip};
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_terminal_protocol::*;

fn cursor(after_sequence: u64) -> OutputCursor {
    OutputCursor {
        target: fixtures::target(),
        after_sequence,
    }
}

#[test]
fn attachment_cursors_cannot_escape_the_requested_runtime() {
    roundtrip(&AttachRequest::new(fixtures::target(), None).unwrap());
    let request = AttachRequest::new(fixtures::target(), Some(cursor(TEST_SEQUENCE))).unwrap();
    roundtrip(&request);
    assert_eq!(request.cursor(), Some(cursor(TEST_SEQUENCE)));
    let mut wrong = cursor(TEST_SEQUENCE);
    wrong.target.runtime_epoch = RuntimeEpoch::new();
    assert_eq!(
        AttachRequest::new(fixtures::target(), Some(wrong)),
        Err(PublicError::TargetMismatch)
    );
    assert_eq!(
        AttachRequest::new(fixtures::target(), Some(cursor(u64::MAX))),
        Err(PublicError::InvalidCursor)
    );
}

#[test]
fn snapshot_or_contiguous_replay_advances_exact_cursor() {
    let result = AttachResult::new(fixtures::replay_fields()).unwrap();
    let request =
        AttachRequest::new(fixtures::target(), Some(cursor(TEST_SEQUENCE - STEP))).unwrap();
    assert_eq!(
        result.validate_for_request(request),
        Ok(cursor(TEST_SEQUENCE))
    );
    roundtrip(&result);
    assert_eq!(result.fields().next_sequence, TEST_SEQUENCE + STEP);
    let result = AttachResult::new(fixtures::snapshot_result_fields()).unwrap();
    assert_eq!(
        result.validate_for_request(request),
        Ok(cursor(TEST_SEQUENCE))
    );
    assert_eq!(
        result.validate_for_request(AttachRequest::new(fixtures::target(), None).unwrap()),
        Ok(cursor(TEST_SEQUENCE))
    );
    roundtrip(&result);
    let fields: AttachResultFields = result.into();
    assert!(fields.resynchronized);
}

#[test]
fn empty_replay_is_valid_only_at_the_matching_cursor() {
    let mut fields = fixtures::replay_fields();
    fields.chunks.clear();
    let result = AttachResult::new(fields).unwrap();
    assert_eq!(
        result.validate_for_request(
            AttachRequest::new(fixtures::target(), Some(cursor(TEST_SEQUENCE))).unwrap()
        ),
        Ok(cursor(TEST_SEQUENCE))
    );
    assert_eq!(
        result.validate_for_request(
            AttachRequest::new(fixtures::target(), Some(cursor(TEST_SEQUENCE - STEP))).unwrap()
        ),
        Err(PublicError::OutputGap)
    );
    assert_eq!(
        result.validate_for_request(AttachRequest::new(fixtures::target(), None).unwrap()),
        Err(PublicError::OutputGap)
    );
}

#[test]
fn replay_and_snapshot_must_match_request_not_just_self_consistency() {
    let replay = AttachResult::new(fixtures::replay_fields()).unwrap();
    let wrong = AttachRequest::new(fixtures::target(), Some(cursor(TEST_SEQUENCE))).unwrap();
    assert_eq!(
        replay.validate_for_request(wrong),
        Err(PublicError::OutputGap)
    );
    let mut target = fixtures::target();
    target.runtime_epoch = RuntimeEpoch::new();
    assert_eq!(
        replay.validate_for_request(AttachRequest::new(target, None).unwrap()),
        Err(PublicError::TargetMismatch)
    );
    let snapshot = AttachResult::new(fixtures::snapshot_result_fields()).unwrap();
    assert_eq!(
        snapshot.validate_for_request(
            AttachRequest::new(fixtures::target(), Some(cursor(TEST_SEQUENCE + STEP))).unwrap()
        ),
        Err(PublicError::OutputGap)
    );
}

#[test]
fn result_rejects_wrong_epochs_gaps_duplicates_and_mixed_snapshots() {
    let mutations: &[fn(&mut AttachResultFields)] = &[
        |f| f.next_sequence = 0,
        |f| f.next_sequence += STEP,
        |f| f.resynchronized = true,
        |f| f.target.runtime_epoch = RuntimeEpoch::new(),
        |f| f.chunks.push(f.chunks[0].clone()),
        |f| {
            f.chunks.push(
                OutputChunk::new(fixtures::chunk_fields(TEST_SEQUENCE + STEP + STEP)).unwrap(),
            )
        },
    ];
    for mutate in mutations {
        let mut fields = fixtures::replay_fields();
        mutate(&mut fields);
        assert!(AttachResult::new(fields).is_err());
    }
    let mutations: &[fn(&mut AttachResultFields)] = &[
        |f| f.resynchronized = false,
        |f| f.next_sequence += STEP,
        |f| f.target.runtime_epoch = RuntimeEpoch::new(),
        |f| {
            f.chunks
                .push(OutputChunk::new(fixtures::chunk_fields(TEST_SEQUENCE)).unwrap())
        },
    ];
    for mutate in mutations {
        let mut fields = fixtures::snapshot_result_fields();
        mutate(&mut fields);
        assert!(AttachResult::new(fields).is_err());
    }
}

#[test]
fn replay_count_and_aggregate_payload_are_bounded() {
    let mut fields = fixtures::replay_fields();
    fields.chunks = vec![fields.chunks[0].clone(); MAX_ATTACH_CHUNKS + STEP as usize];
    assert_eq!(AttachResult::new(fields), Err(PublicError::InvalidPayload));
    let mut fields = fixtures::replay_fields();
    fields.chunks.clear();
    let count = MAX_ATTACH_OUTPUT_BYTES / MAX_OUTPUT_CHUNK_BYTES;
    for index in 0..=count {
        let mut chunk = fixtures::chunk_fields(FIRST_SEQUENCE + index as u64);
        chunk.data = vec![0; MAX_OUTPUT_CHUNK_BYTES];
        fields.chunks.push(OutputChunk::new(chunk).unwrap());
    }
    fields.next_sequence = FIRST_SEQUENCE + fields.chunks.len() as u64;
    assert_eq!(
        AttachResult::new(fields.clone()),
        Err(PublicError::InvalidPayload)
    );
    fields.chunks.pop();
    fields.next_sequence -= STEP;
    assert!(AttachResult::new(fields).is_ok());
}
