use super::super::constants::error_code;
use crate::client::ClientError;
use crate::lease::constants::code::SESSION_NOT_RUNNING;

pub(super) fn is_ownership_error(error: &ClientError) -> bool {
    matches!(error, ClientError::Remote { code, .. } if matches!(code.as_str(),
        error_code::HELD_BY_OTHER | error_code::LEASE_EXPIRED | error_code::NOT_HOLDER
        | error_code::STALE_GENERATION | error_code::CONTROLLER_INSTANCE_MISMATCH))
}

pub(in crate::relay) fn has_session_not_running_code(error: &ClientError) -> bool {
    matches!(error, ClientError::Remote { code, .. } if code == SESSION_NOT_RUNNING)
}

pub(in crate::relay) fn is_session_not_running(error: &ClientError) -> bool {
    matches!(error, ClientError::Remote {
        code,
        retryable: false,
        details: None,
        ..
    } if code == SESSION_NOT_RUNNING)
}
