use std::time::Duration;

pub const CONNECTION_BOOTSTRAP_TIMEOUT: Duration =
    crate::session_timing::REMOTE_CONNECTION_BOOTSTRAP_TIMEOUT;
pub const MANAGEMENT_OPERATION_TIMEOUT: Duration = CONNECTION_BOOTSTRAP_TIMEOUT
    .saturating_add(crate::session_timing::REMOTE_MANAGEMENT_RESPONSE_MARGIN);
pub const BROKER_READY_TIMEOUT: Duration = MANAGEMENT_OPERATION_TIMEOUT;
pub const BROKER_CLIENT_OPERATION_TIMEOUT: Duration = MANAGEMENT_OPERATION_TIMEOUT
    .saturating_add(crate::session_timing::REMOTE_MANAGEMENT_RESPONSE_MARGIN);
pub const RUNTIME_OPEN_TIMEOUT: Duration = CONNECTION_BOOTSTRAP_TIMEOUT
    .saturating_add(crate::remote::SSH_OPERATION_TIMEOUT)
    .saturating_add(crate::session_timing::REMOTE_RUNTIME_OPEN_SETTLEMENT_MARGIN);
pub const EXISTING_SOCKET_CLIENT_OPERATION_TIMEOUT: Duration = RUNTIME_OPEN_TIMEOUT
    .saturating_add(crate::session_timing::REMOTE_EXISTING_RUNTIME_INNER_BUDGET)
    .saturating_add(crate::session_timing::REMOTE_OPERATION_RESPONSE_MARGIN);

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/timing_inline_tests.rs"]
mod tests;
