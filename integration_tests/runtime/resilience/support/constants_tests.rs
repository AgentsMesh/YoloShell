use super::*;

#[test]
fn cleanup_deadlines_cover_the_worker_protocol_contract() {
    assert!(CLEANUP_RPC_TIMEOUT >= yolo_session_timing::WORKER_AUTH_TIMEOUT);
    assert!(
        RUNTIME_GRACEFUL_EXIT_TIMEOUT >= yolo_session_timing::WORKER_TERMINATION_PROTOCOL_TIMEOUT
    );
}
