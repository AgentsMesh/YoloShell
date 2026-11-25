use crate::fixtures::{self, constants::*, roundtrip};
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_terminal_protocol::*;

#[test]
fn reconnect_replays_contiguous_output_then_resynchronizes_after_retention_gap() {
    let attach = AttachRequest::new(fixtures::target(), None).unwrap();
    let snapshot = AttachResult::new(fixtures::snapshot_result_fields()).unwrap();
    let cursor = snapshot.validate_for_request(attach).unwrap();
    let request = AttachRequest::new(fixtures::target(), Some(cursor)).unwrap();
    let mut fields = fixtures::replay_fields();
    fields.chunks = vec![OutputChunk::new(fixtures::chunk_fields(TEST_SEQUENCE + STEP)).unwrap()];
    fields.next_sequence = TEST_SEQUENCE + STEP + STEP;
    let replay = AttachResult::new(fields).unwrap();
    let cursor = replay.validate_for_request(request).unwrap();
    assert_eq!(cursor.after_sequence, TEST_SEQUENCE + STEP);
    let request = AttachRequest::new(fixtures::target(), Some(cursor)).unwrap();
    let mut fields = fixtures::snapshot_result_fields();
    fields.snapshot = Some(
        TerminalSnapshot::new(fixtures::snapshot_fields(TEST_SEQUENCE + TEST_SEQUENCE)).unwrap(),
    );
    fields.next_sequence = TEST_SEQUENCE + TEST_SEQUENCE + STEP;
    let recovered = AttachResult::new(fields).unwrap();
    assert_eq!(
        recovered
            .validate_for_request(request)
            .unwrap()
            .after_sequence,
        TEST_SEQUENCE + TEST_SEQUENCE
    );
    roundtrip(&recovered);
}

#[test]
fn runtime_recreation_invalidates_old_cursor_and_lifecycle_selection() {
    let old_target = fixtures::target();
    let mut fields = fixtures::summary_fields();
    fields.runtime.as_mut().unwrap().target.runtime_epoch = RuntimeEpoch::new();
    let current = TerminalSummary::new(fields).unwrap();
    let old_selection = TerminalSelection {
        address: old_target.address,
        expected_runtime: Some(old_target.runtime_epoch),
    };
    assert_eq!(
        old_selection.validate_against(&current),
        Err(PublicError::TargetMismatch)
    );
    let new_target = current.fields().runtime.unwrap().target;
    let old_cursor = OutputCursor {
        target: old_target,
        after_sequence: TEST_SEQUENCE,
    };
    assert_eq!(
        AttachRequest::new(new_target, Some(old_cursor)),
        Err(PublicError::TargetMismatch)
    );
    let old_input = InputRequest::new(fixtures::input_fields()).unwrap();
    assert_ne!(old_input.fields().grant.fields().target, new_target);
}

#[test]
fn stable_creation_identity_survives_a_new_rpc_identity() {
    let first = CreateShellRequest::new(fixtures::create_fields()).unwrap();
    let mut fields = first.clone().into();
    let fields: &mut CreateShellFields = &mut fields;
    fields.request_id = uuid::Uuid::new_v4();
    let retry = CreateShellRequest::new(fields.clone()).unwrap();
    assert_ne!(first.fields().request_id, retry.fields().request_id);
    assert_eq!(first.fields().creation_token, retry.fields().creation_token);
    assert_eq!(first.fields().cwd, retry.fields().cwd);
    roundtrip(&first);
    roundtrip(&retry);
}
