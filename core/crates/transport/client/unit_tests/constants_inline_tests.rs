use super::*;

const LONGER_CONFIGURED_TIMEOUT_MARGIN: Duration = Duration::from_secs(15);

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn session_startup_timeout_applies_its_floor_to_create() {
    assert!(
        DEFAULT_CLIENT_OPERATION_TIMEOUT < crate::session_timing::SESSION_STARTUP_CLIENT_TIMEOUT
    );
    assert_eq!(
        request_operation_timeout(
            DEFAULT_CLIENT_OPERATION_TIMEOUT,
            crate::protocol::method::CREATE_SESSION
        ),
        crate::session_timing::SESSION_STARTUP_CLIENT_TIMEOUT
    );
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn session_resume_timeout_covers_predecessor_settlement_and_startup() {
    assert!(
        crate::session_timing::SESSION_STARTUP_CLIENT_TIMEOUT
            < crate::session_timing::SESSION_RESUME_CLIENT_TIMEOUT
    );
    assert_eq!(
        request_operation_timeout(
            DEFAULT_CLIENT_OPERATION_TIMEOUT,
            crate::protocol::method::RESUME_SESSION_EXACT,
        ),
        crate::session_timing::SESSION_RESUME_CLIENT_TIMEOUT
    );
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn session_startup_policy_leaves_explicit_reconcile_at_configured_timeout() {
    assert_eq!(
        request_operation_timeout(
            DEFAULT_CLIENT_OPERATION_TIMEOUT,
            crate::protocol::method::RECONCILE
        ),
        DEFAULT_CLIENT_OPERATION_TIMEOUT
    );
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn session_lifecycle_timeout_applies_to_every_lifecycle_method() {
    assert!(
        DEFAULT_CLIENT_OPERATION_TIMEOUT < crate::session_timing::SESSION_LIFECYCLE_CLIENT_TIMEOUT
    );
    for method in [
        crate::protocol::method::TERMINATE_SESSION,
        crate::protocol::method::ARCHIVE_SESSION,
        crate::protocol::method::DISCARD_SESSION,
        crate::protocol::method::CLEANUP_TERMINATE_SESSION,
        crate::protocol::method::CLEANUP_ARCHIVE_SESSION,
        crate::protocol::method::CLEANUP_DISCARD_SESSION,
        crate::protocol::method::FINALIZE_CREATION,
    ] {
        assert_eq!(
            request_operation_timeout(DEFAULT_CLIENT_OPERATION_TIMEOUT, method),
            crate::session_timing::SESSION_LIFECYCLE_CLIENT_TIMEOUT
        );
    }
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn session_startup_timeout_preserves_equal_configuration() {
    let configured = crate::session_timing::SESSION_STARTUP_CLIENT_TIMEOUT;
    assert_eq!(
        request_operation_timeout(configured, crate::protocol::method::CREATE_SESSION),
        configured
    );
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn session_startup_timeout_preserves_longer_configuration() {
    let configured = crate::session_timing::SESSION_RESUME_CLIENT_TIMEOUT
        .saturating_add(LONGER_CONFIGURED_TIMEOUT_MARGIN);
    assert_eq!(
        request_operation_timeout(configured, crate::protocol::method::RESUME_SESSION_EXACT),
        configured
    );
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn session_lifecycle_timeout_preserves_longer_configuration() {
    let configured = crate::session_timing::SESSION_LIFECYCLE_CLIENT_TIMEOUT
        .saturating_add(LONGER_CONFIGURED_TIMEOUT_MARGIN);
    assert_eq!(
        request_operation_timeout(configured, crate::protocol::method::DISCARD_SESSION),
        configured
    );
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn session_startup_policy_leaves_ordinary_requests_at_configured_timeout() {
    assert_eq!(
        request_operation_timeout(
            DEFAULT_CLIENT_OPERATION_TIMEOUT,
            crate::protocol::method::HOST_STATUS
        ),
        DEFAULT_CLIENT_OPERATION_TIMEOUT
    );
}
