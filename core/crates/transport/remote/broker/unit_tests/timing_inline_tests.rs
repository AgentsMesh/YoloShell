use super::*;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn outer_deadlines_leave_bootstrap_and_response_margin() {
    assert!(CONNECTION_BOOTSTRAP_TIMEOUT < MANAGEMENT_OPERATION_TIMEOUT);
    assert_eq!(BROKER_READY_TIMEOUT, MANAGEMENT_OPERATION_TIMEOUT);
    assert!(BROKER_READY_TIMEOUT < BROKER_CLIENT_OPERATION_TIMEOUT);
    assert!(BROKER_CLIENT_OPERATION_TIMEOUT < RUNTIME_OPEN_TIMEOUT);
    assert!(
        CONNECTION_BOOTSTRAP_TIMEOUT + crate::remote::SSH_OPERATION_TIMEOUT < RUNTIME_OPEN_TIMEOUT
    );
    assert!(RUNTIME_OPEN_TIMEOUT < EXISTING_SOCKET_CLIENT_OPERATION_TIMEOUT);
    assert_eq!(
        EXISTING_SOCKET_CLIENT_OPERATION_TIMEOUT,
        RUNTIME_OPEN_TIMEOUT
            + crate::session_timing::REMOTE_EXISTING_RUNTIME_INNER_BUDGET
            + crate::session_timing::REMOTE_OPERATION_RESPONSE_MARGIN
    );
    assert!(
        crate::session_timing::SESSION_LIFECYCLE_INNER_BUDGET
            < crate::session_timing::WORKER_READY_TIMEOUT
    );
    assert!(
        crate::session_timing::SESSION_LIFECYCLE_CLIENT_TIMEOUT
            < EXISTING_SOCKET_CLIENT_OPERATION_TIMEOUT
    );
}
