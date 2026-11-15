use super::*;
use crate::client::ClientError;
use crate::lease::constants::code;
use crate::protocol::error_code;
use crate::relay::constants::test_value;

const UNTRUSTED_MESSAGE: &str = "untrusted terminal error";

fn remote(
    code: &str,
    message: &str,
    retryable: bool,
    details: Option<serde_json::Value>,
) -> ClientError {
    ClientError::Remote {
        code: code.into(),
        message: message.into(),
        retryable,
        details,
    }
}

#[test]
fn terminal_code_matching_is_exact_and_requires_canonical_metadata() {
    let message_only_code = format!(
        "{}{}",
        test_value::NON_MATCHING_CODE_PREFIX,
        code::SESSION_NOT_RUNNING
    );
    let similar_code = format!(
        "{}{}",
        code::SESSION_NOT_RUNNING,
        test_value::SIMILAR_CODE_SUFFIX
    );
    let exact = remote(code::SESSION_NOT_RUNNING, UNTRUSTED_MESSAGE, false, None);
    let retryable = remote(code::SESSION_NOT_RUNNING, UNTRUSTED_MESSAGE, true, None);
    let detailed = remote(
        code::SESSION_NOT_RUNNING,
        UNTRUSTED_MESSAGE,
        false,
        Some(serde_json::Value::Bool(true)),
    );
    let message_only = remote(
        &message_only_code,
        code::SESSION_NOT_RUNNING,
        false,
        None,
    );
    let similar = remote(&similar_code, UNTRUSTED_MESSAGE, false, None);

    assert!(is_session_not_running(&exact));
    assert!(!is_session_not_running(&retryable));
    assert!(!is_session_not_running(&detailed));
    assert!(!is_session_not_running(&message_only));
    assert!(!is_session_not_running(&similar));
    assert!(has_session_not_running_code(&retryable));
    assert!(!super::super::client_error_is_retryable(&retryable));
}

#[test]
fn unrelated_authority_and_internal_errors_never_look_terminal() {
    for other in [
        error_code::IDENTITY_MISMATCH,
        error_code::UNAUTHORIZED,
        error_code::INTERNAL,
    ] {
        let error = remote(other, UNTRUSTED_MESSAGE, false, None);
        assert!(!has_session_not_running_code(&error));
        assert!(!is_session_not_running(&error));
    }
}
