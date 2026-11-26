use crate::host::tests::*;
use super::*;

const COMPLETED_SNAPSHOT_SEQUENCE_OFFSET: u64 = 1;

#[test]
fn completed_snapshot_requires_the_exact_exit_epoch_and_barrier() {
    let runtime_epoch = Uuid::new_v4();
    let exit = ExitRecord {
        runtime_epoch,
        code: Some(COMPLETED_TEST_EXIT_CODE),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: COMPLETED_TEST_OBSERVED_AT_MS,
        last_output_sequence: COMPLETED_TEST_OUTPUT_SEQUENCE,
    };
    let exact = crate::model::TerminalSnapshot {
        runtime_epoch,
        barrier_sequence: COMPLETED_TEST_OUTPUT_SEQUENCE,
        cols: TEST_TERMINAL_COLUMNS,
        rows: TEST_TERMINAL_ROWS,
        data: COMPLETED_TEST_SNAPSHOT.to_vec(),
    };
    assert!(validate_completed_snapshot(&snapshot_response(&exact), &exit).is_ok());

    let mut wrong_epoch = exact.clone();
    wrong_epoch.runtime_epoch = Uuid::new_v4();
    assert_snapshot_identity_mismatch(&wrong_epoch, &exit);
    let mut lower_barrier = exact.clone();
    lower_barrier.barrier_sequence = COMPLETED_TEST_OUTPUT_SEQUENCE
        .checked_sub(COMPLETED_SNAPSHOT_SEQUENCE_OFFSET)
        .unwrap();
    assert_snapshot_identity_mismatch(&lower_barrier, &exit);
    let mut higher_barrier = exact;
    higher_barrier.barrier_sequence = COMPLETED_TEST_OUTPUT_SEQUENCE
        .checked_add(COMPLETED_SNAPSHOT_SEQUENCE_OFFSET)
        .unwrap();
    assert_snapshot_identity_mismatch(&higher_barrier, &exit);
}

#[test]
fn completed_snapshot_rejects_a_malformed_success_payload() {
    let runtime_epoch = Uuid::new_v4();
    let exit = ExitRecord {
        runtime_epoch,
        code: Some(COMPLETED_TEST_EXIT_CODE),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: COMPLETED_TEST_OBSERVED_AT_MS,
        last_output_sequence: COMPLETED_TEST_OUTPUT_SEQUENCE,
    };

    let error = validate_completed_snapshot(&snapshot_response(&()), &exit).unwrap_err();

    assert_eq!(
        error.code,
        crate::protocol::error_code::PROTOCOL_INCOMPATIBLE
    );
    assert_eq!(error.message, COMPLETED_SNAPSHOT_ATTESTATION_MISMATCH);
    assert!(!error.retryable);
}

fn assert_snapshot_identity_mismatch(snapshot: &crate::model::TerminalSnapshot, exit: &ExitRecord) {
    let error = validate_completed_snapshot(&snapshot_response(snapshot), exit).unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert_eq!(error.message, COMPLETED_SNAPSHOT_ATTESTATION_MISMATCH);
    assert!(!error.retryable);
}

fn snapshot_response<T: serde::Serialize>(payload: &T) -> ResponseEnvelope {
    let request = RequestEnvelope::new(
        TEST_REQUEST_ID,
        method::GET_SNAPSHOT,
        yolo_capnp_value::Value::Null,
    );
    ResponseEnvelope::success(&request, payload).unwrap()
}
