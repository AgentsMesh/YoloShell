use crate::native_diagnostics::RpcResult;
use yolo_client::ClientError;
use yolo_runtime::{
    lease::constants::code as lease_code,
    protocol::{ProtocolError, error_code},
    worker::NativeControlOutcome,
};
#[path = "../test_support/native_diagnostics.rs"]
mod support;
use support::*;

#[test]
fn known_remote_codes_remain_diagnostic_without_exposing_message_or_details() {
    for code in [
        error_code::CONFLICT,
        error_code::IDENTITY_MISMATCH,
        error_code::INTERNAL,
        error_code::WORKER_UNREACHABLE,
        lease_code::LEASE_EXPIRED,
        lease_code::STALE_GENERATION,
        lease_code::NOT_HOLDER,
    ] {
        let classified = RpcResult::client_error(&remote(code));
        assert_eq!(classified, RpcResult::Remote { code: Some(code) });
        assert!(!format!("{classified:?}").contains(PRIVATE_MARKER));
    }
    let unrecognized = RpcResult::client_error(&remote(PRIVATE_MARKER));
    assert_eq!(unrecognized, RpcResult::Remote { code: None });
    assert!(!format!("{unrecognized:?}").contains(PRIVATE_MARKER));
}

#[test]
fn client_failures_retain_only_their_class_and_response_mismatch_is_distinct() {
    for (error, expected) in failures() {
        let actual = RpcResult::client_error(&error);
        assert_eq!(actual, expected);
        assert!(!format!("{actual:?}").contains(PRIVATE_MARKER));
    }
    assert_eq!(
        RpcResult::client_error(&ClientError::Deadline),
        RpcResult::Deadline
    );
    assert_eq!(
        RpcResult::client_error(&ClientError::Protocol(ProtocolError::ResponseMismatch)),
        RpcResult::CorrelationMismatch,
    );
}

#[test]
fn worker_unknown_is_distinct_from_lookup_absence_and_transport_failure() {
    for (outcome, expected) in [
        (NativeControlOutcome::Delivered, RpcResult::Delivered),
        (NativeControlOutcome::NotDelivered, RpcResult::NotDelivered),
        (NativeControlOutcome::Unknown, RpcResult::Unknown),
    ] {
        let actual = RpcResult::from(outcome);
        assert_eq!(actual, expected);
        assert_ne!(actual, RpcResult::Missing);
        assert_ne!(actual, RpcResult::Deadline);
    }
}
