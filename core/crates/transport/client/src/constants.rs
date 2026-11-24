use std::time::Duration;
pub(crate) const HOST_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) const DEFAULT_CLIENT_OPERATION_TIMEOUT: Duration =
    crate::session_timing::RUST_INSPECTION_CLIENT_TIMEOUT;

pub(super) fn request_operation_timeout(configured: Duration, method_name: &str) -> Duration {
    match method_name {
        crate::protocol::method::CREATE_SESSION => {
            configured.max(crate::session_timing::SESSION_STARTUP_CLIENT_TIMEOUT)
        }
        crate::protocol::method::RESUME_SESSION_EXACT => {
            configured.max(crate::session_timing::SESSION_RESUME_CLIENT_TIMEOUT)
        }
        crate::protocol::method::TERMINATE_SESSION
        | crate::protocol::method::ARCHIVE_SESSION
        | crate::protocol::method::DISCARD_SESSION
        | crate::protocol::method::CLEANUP_TERMINATE_SESSION
        | crate::protocol::method::CLEANUP_ARCHIVE_SESSION
        | crate::protocol::method::CLEANUP_DISCARD_SESSION
        | crate::protocol::method::FINALIZE_CREATION => {
            configured.max(crate::session_timing::SESSION_LIFECYCLE_CLIENT_TIMEOUT)
        }
        _ => configured,
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/constants_inline_tests.rs"]
mod tests;
