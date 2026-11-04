use crate::host::tests::*;
use super::*;

const LEGACY_CURSOR_SOCKET_NAME: &str = "legacy-cursor.sock";
const LEGACY_CURSOR_AFTER_SEQUENCE: u64 = 9;
const LEGACY_CURSOR_BARRIER_SEQUENCE: u64 = 7;

#[test]
fn invalid_ahead_cursor_snapshot_is_rejected_as_a_protocol_violation() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let record = host_runtime_record(
        &host,
        terminal_id,
        directory.path().join(LEGACY_CURSOR_SOCKET_NAME),
    );
    let attach = AttachRequest {
        terminal_id,
        after_seq: LEGACY_CURSOR_AFTER_SEQUENCE,
    };
    let request = request(method::ATTACH_SESSION, &attach);
    let snapshot = crate::model::TerminalSnapshot {
        runtime_epoch: record.runtime_epoch,
        barrier_sequence: LEGACY_CURSOR_BARRIER_SEQUENCE,
        cols: TEST_TERMINAL_COLUMNS,
        rows: TEST_TERMINAL_ROWS,
        data: Vec::new(),
    };
    let response = ResponseEnvelope::success(
        &request,
        &crate::worker::AttachResult {
            runtime_epoch: record.runtime_epoch,
            snapshot: Some(snapshot),
            chunks: Vec::new(),
            next_sequence: crate::worker::attachment_next_sequence(LEGACY_CURSOR_BARRIER_SEQUENCE)
                .unwrap(),
            resynchronized: true,
        },
    )
    .unwrap();

    let error = validate_worker_attachment(&request, &response, &record).unwrap_err();

    assert_eq!(
        error.code,
        crate::protocol::error_code::PROTOCOL_INCOMPATIBLE
    );
    assert_eq!(error.message, WORKER_ATTACHMENT_PROTOCOL_INVALID);
    assert!(!error.retryable);
}
