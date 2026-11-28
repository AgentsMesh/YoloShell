use super::super::settle_disconnect;
use crate::remote::RemoteError;

const CLEANUP_FAILURE: &str = "injected cleanup failure";
const DISCONNECT_FAILURE: &str = "injected disconnect failure";

#[test]
fn successful_cleanup_preserves_the_disconnect_result() {
    assert!(settle_disconnect(Ok(()), Ok(())).is_ok());
    assert_transport(
        settle_disconnect(Ok(()), transport(DISCONNECT_FAILURE)).unwrap_err(),
        DISCONNECT_FAILURE,
    );
}

#[test]
fn failed_cleanup_is_primary_when_disconnect_succeeds() {
    assert_transport(
        settle_disconnect(transport(CLEANUP_FAILURE), Ok(())).unwrap_err(),
        CLEANUP_FAILURE,
    );
}

#[test]
fn dual_failure_preserves_cleanup_and_disconnect_evidence() {
    let error =
        settle_disconnect(transport(CLEANUP_FAILURE), transport(DISCONNECT_FAILURE)).unwrap_err();
    let RemoteError::LifecycleSettlement {
        primary,
        settlement,
    } = error
    else {
        unreachable!()
    };
    assert_transport(*primary, CLEANUP_FAILURE);
    assert_transport(*settlement, DISCONNECT_FAILURE);
}

fn transport(message: &str) -> Result<(), RemoteError> {
    Err(RemoteError::transport(message))
}

fn assert_transport(error: RemoteError, expected: &str) {
    assert!(matches!(error, RemoteError::Transport(detail) if detail == expected));
}
