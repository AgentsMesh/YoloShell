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
fn history_seed_roundtrips_and_advances_to_the_history_barrier() {
    let request = AttachRequest::history(fixtures::target(), None).unwrap();
    roundtrip(&request);
    assert_eq!(request.seed(), AttachSeed::History);
    let result = AttachResult::new(fixtures::history_result_fields()).unwrap();
    assert_eq!(
        result.validate_for_request(request),
        Ok(cursor(TEST_SEQUENCE))
    );
    assert_eq!(
        result.validate_for_request(
            AttachRequest::history(
                fixtures::target(),
                Some(cursor(TEST_SEQUENCE - STEP))
            )
            .unwrap()
        ),
        Ok(cursor(TEST_SEQUENCE))
    );
    assert_eq!(
        result.validate_for_request(
            AttachRequest::history(fixtures::target(), Some(cursor(TEST_SEQUENCE + STEP))).unwrap()
        ),
        Err(PublicError::OutputGap)
    );
    assert_eq!(
        result.validate_for_request(AttachRequest::new(fixtures::target(), None).unwrap()),
        Err(PublicError::OutputGap)
    );
    roundtrip(&result);
    let fields: AttachResultFields = result.into();
    assert!(fields.history.is_some());
}

#[test]
fn history_cannot_mix_with_snapshot_replay_or_resynchronization() {
    let mutations: &[fn(&mut AttachResultFields)] = &[
        |f| f.snapshot = Some(TerminalSnapshot::new(fixtures::snapshot_fields(TEST_SEQUENCE)).unwrap()),
        |f| f.chunks.push(OutputChunk::new(fixtures::chunk_fields(TEST_SEQUENCE)).unwrap()),
        |f| f.resynchronized = true,
        |f| f.next_sequence += STEP,
        |f| f.target.runtime_epoch = RuntimeEpoch::new(),
    ];
    for mutate in mutations {
        let mut fields = fixtures::history_result_fields();
        mutate(&mut fields);
        assert!(AttachResult::new(fields).is_err());
    }
}

#[test]
fn history_payload_and_barrier_are_bounded() {
    let mut fields = fixtures::history_fields(TEST_SEQUENCE);
    fields.data = vec![0; MAX_SNAPSHOT_BYTES + STEP as usize];
    assert_eq!(TerminalHistory::new(fields), Err(PublicError::InvalidPayload));
    let mut fields = fixtures::history_fields(u64::MAX);
    fields.data = Vec::new();
    assert_eq!(TerminalHistory::new(fields), Err(PublicError::InvalidCursor));
    let mut fields = fixtures::history_fields(0);
    fields.data = Vec::new();
    assert!(TerminalHistory::new(fields).is_ok());
}

