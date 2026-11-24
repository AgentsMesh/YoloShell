use crate::fixtures::{self, constants::*, roundtrip};
use yolo_hub_terminal_protocol::*;

#[test]
fn snapshots_allow_empty_state_but_have_bounded_restore_payload() {
    let mut fields = fixtures::snapshot_fields(0);
    fields.data.clear();
    let snapshot = TerminalSnapshot::new(fields).unwrap();
    assert_eq!(snapshot.fields().barrier_sequence, 0);
    roundtrip(&snapshot);
    let mut fields: TerminalSnapshotFields = snapshot.into();
    fields.data = vec![0; MAX_SNAPSHOT_BYTES + STEP as usize];
    assert_eq!(
        TerminalSnapshot::new(fields),
        Err(PublicError::InvalidPayload)
    );
    assert_eq!(
        TerminalSnapshot::new(fixtures::snapshot_fields(u64::MAX)),
        Err(PublicError::InvalidCursor)
    );
    let mut fields = fixtures::snapshot_fields(TEST_SEQUENCE);
    fields.data = vec![0; MAX_SNAPSHOT_BYTES];
    assert!(TerminalSnapshot::new(fields).is_ok());
}

#[test]
fn output_chunks_require_positive_nonexhausted_sequence_and_bounded_data() {
    let chunk = OutputChunk::new(fixtures::chunk_fields(FIRST_SEQUENCE)).unwrap();
    assert_eq!(chunk.fields().data, OUTPUT);
    roundtrip(&chunk);
    let fields: OutputChunkFields = chunk.into();
    assert_eq!(fields.sequence, FIRST_SEQUENCE);
    for sequence in [0, u64::MAX] {
        assert_eq!(
            OutputChunk::new(fixtures::chunk_fields(sequence)),
            Err(PublicError::InvalidCursor)
        );
    }
    for length in [0, MAX_OUTPUT_CHUNK_BYTES + STEP as usize] {
        let mut fields = fixtures::chunk_fields(TEST_SEQUENCE);
        fields.data = vec![0; length];
        assert_eq!(OutputChunk::new(fields), Err(PublicError::InvalidPayload));
    }
    let mut fields = fixtures::chunk_fields(TEST_SEQUENCE);
    fields.data = vec![0; MAX_OUTPUT_CHUNK_BYTES];
    assert!(OutputChunk::new(fields).is_ok());
}
