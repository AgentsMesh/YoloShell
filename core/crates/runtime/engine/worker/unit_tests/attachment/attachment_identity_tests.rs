use super::*;
use crate::model::{OutputChunk, TerminalSnapshot};
use crate::protocol::{field, from_value, to_value};
use yolo_capnp_value::Value;

const NEXT_SEQUENCE: u64 = 1;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_ROWS: u16 = 24;
const REPLAY_AFTER_SEQUENCE: u64 = 7;
const SEQUENCE_INCREMENT: u64 = 1;
const EXHAUSTED_SEQUENCE: u64 = u64::MAX;
const MALFORMED_EPOCH: &str = "not-a-runtime-epoch";

fn empty(runtime_epoch: Uuid) -> AttachResult {
    AttachResult {
        runtime_epoch,
        snapshot: None,
        chunks: Vec::new(),
        next_sequence: NEXT_SEQUENCE,
        resynchronized: false,
    }
}

#[test]
fn attachment_epoch_empty_result_requires_non_nil_identity() {
    let epoch = Uuid::new_v4();
    empty(epoch).validate_runtime_epoch(epoch).unwrap();
    assert!(matches!(
        empty(Uuid::nil()).validate_runtime_epoch(Uuid::nil()),
        Err(ProtocolError::AttachmentMismatch)
    ));
}

#[test]
fn attachment_epoch_missing_malformed_and_duplicate_fields_are_rejected() {
    let Value::Map(fields) = to_value(&empty(Uuid::new_v4())).unwrap() else {
        panic!("attach result must encode as a map");
    };
    let without_epoch = fields
        .iter()
        .filter(|(key, _)| key.as_str() != Some(field::RUNTIME_EPOCH))
        .cloned()
        .collect();
    assert!(from_value::<AttachResult>(Value::Map(without_epoch)).is_err());
    let mut malformed = fields.clone();
    malformed.retain(|(key, _)| key.as_str() != Some(field::RUNTIME_EPOCH));
    malformed.push((
        Value::from(field::RUNTIME_EPOCH),
        Value::from(MALFORMED_EPOCH),
    ));
    assert!(from_value::<AttachResult>(Value::Map(malformed)).is_err());
    let mut duplicate = fields;
    duplicate.push((
        Value::from(field::RUNTIME_EPOCH),
        Value::from(Uuid::new_v4().to_string()),
    ));
    assert!(from_value::<AttachResult>(Value::Map(duplicate)).is_err());
}

#[test]
fn attachment_epoch_initial_snapshot_and_chunks_must_match() {
    let expected = Uuid::new_v4();
    let wrong = Uuid::new_v4();
    let mut attached = empty(expected);
    attached.snapshot = Some(TerminalSnapshot {
        runtime_epoch: wrong,
        barrier_sequence: NEXT_SEQUENCE,
        cols: TERMINAL_COLUMNS,
        rows: TERMINAL_ROWS,
        data: Vec::new(),
    });
    assert!(attached.validate_runtime_epoch(expected).is_err());
    attached.snapshot = None;
    attached.chunks.push(OutputChunk {
        runtime_epoch: wrong,
        sequence: NEXT_SEQUENCE,
        data: Vec::new(),
    });
    assert!(attached.validate_runtime_epoch(expected).is_err());
}

#[test]
fn attachment_replay_binds_request_chunks_and_next_sequence() {
    let epoch = Uuid::new_v4();
    let first = REPLAY_AFTER_SEQUENCE + SEQUENCE_INCREMENT;
    let second = first + SEQUENCE_INCREMENT;
    let mut attached = empty(epoch);
    attached.chunks = [first, second]
        .map(|sequence| OutputChunk {
            runtime_epoch: epoch,
            sequence,
            data: Vec::new(),
        })
        .into();
    attached.next_sequence = second + SEQUENCE_INCREMENT;
    assert_eq!(
        attached
            .validate_for_request(epoch, REPLAY_AFTER_SEQUENCE)
            .unwrap(),
        second
    );
    attached.chunks[0].sequence = second;
    assert!(
        attached
            .validate_for_request(epoch, REPLAY_AFTER_SEQUENCE)
            .is_err()
    );
    attached.chunks.clear();
    assert!(
        attached
            .validate_for_request(epoch, REPLAY_AFTER_SEQUENCE)
            .is_err()
    );
}

#[test]
fn attachment_force_snapshot_cursor_rejects_a_replay_response() {
    let epoch = Uuid::new_v4();
    let mut attached = empty(epoch);
    attached.next_sequence = EXHAUSTED_SEQUENCE;

    assert!(
        attached
            .validate_for_request(epoch, EXHAUSTED_SEQUENCE)
            .is_err()
    );
}

#[test]
fn attachment_snapshot_binds_barrier_shape_and_resynchronization() {
    const INITIAL_SEQUENCE: u64 = 0;

    let epoch = Uuid::new_v4();
    let mut attached = empty(epoch);
    attached.snapshot = Some(TerminalSnapshot {
        runtime_epoch: epoch,
        barrier_sequence: REPLAY_AFTER_SEQUENCE,
        cols: TERMINAL_COLUMNS,
        rows: TERMINAL_ROWS,
        data: Vec::new(),
    });
    attached.next_sequence = REPLAY_AFTER_SEQUENCE + SEQUENCE_INCREMENT;
    attached.resynchronized = true;
    assert_eq!(
        attached
            .validate_for_request(epoch, INITIAL_SEQUENCE)
            .unwrap(),
        REPLAY_AFTER_SEQUENCE
    );
    assert_eq!(
        attached
            .validate_for_request(epoch, FORCE_SNAPSHOT_AFTER_SEQUENCE)
            .unwrap(),
        REPLAY_AFTER_SEQUENCE
    );
    assert!(
        attached
            .validate_for_request(epoch, REPLAY_AFTER_SEQUENCE + SEQUENCE_INCREMENT)
            .is_err()
    );
    attached.resynchronized = false;
    assert!(
        attached
            .validate_for_request(epoch, INITIAL_SEQUENCE)
            .is_err()
    );
}

#[test]
fn attachment_snapshot_rejects_an_exhausted_barrier() {
    let epoch = Uuid::new_v4();
    let mut attached = empty(epoch);
    attached.snapshot = Some(TerminalSnapshot {
        runtime_epoch: epoch,
        barrier_sequence: EXHAUSTED_SEQUENCE,
        cols: TERMINAL_COLUMNS,
        rows: TERMINAL_ROWS,
        data: Vec::new(),
    });
    attached.next_sequence = EXHAUSTED_SEQUENCE;
    attached.resynchronized = true;

    assert!(
        attached
            .validate_for_request(epoch, REPLAY_AFTER_SEQUENCE)
            .is_err()
    );
}
